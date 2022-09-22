use std::{ffi::CString, path::Path, sync::Arc, time::Duration};

use crate::{
    datastore::{
        lmdb::{mdb_env_copy, MDB_SUCCESS},
        Store, Transaction, VersionStore, WriteTransaction, STORE_VERSION_CURRENT,
        STORE_VERSION_MINIMUM,
    },
    logger_mt::Logger,
    utils::{seconds_since_epoch, PropertyTreeWriter, Serialize},
    LmdbConfig, PendingKey, TxnTrackingConfig,
};

use super::{
    ensure_success, get_raw_lmdb_txn, mdb_count, mdb_dbi_close, mdb_dbi_open, mdb_drop,
    mdb_env_copy2, mdb_env_stat, mdb_put, EnvOptions, LmdbAccountStore, LmdbBlockStore,
    LmdbConfirmationHeightStore, LmdbEnv, LmdbFinalVoteStore, LmdbFrontierStore,
    LmdbOnlineWeightStore, LmdbPeerStore, LmdbPendingStore, LmdbPrunedStore, LmdbRawIterator,
    LmdbUncheckedStore, LmdbVersionStore, MdbStat, MdbVal, MDB_APPEND, MDB_CP_COMPACT, MDB_CREATE,
};

#[derive(PartialEq, Eq)]
pub enum Vacuuming {
    Needed,
    NotNeeded,
}

pub struct LmdbStore {
    pub env: Arc<LmdbEnv>,
    pub block_store: Arc<LmdbBlockStore>,
    pub frontier_store: Arc<LmdbFrontierStore>,
    pub account_store: Arc<LmdbAccountStore>,
    pub pending_store: Arc<LmdbPendingStore>,
    pub online_weight_store: Arc<LmdbOnlineWeightStore>,
    pub pruned_store: Arc<LmdbPrunedStore>,
    pub peer_store: Arc<LmdbPeerStore>,
    pub confirmation_height_store: Arc<LmdbConfirmationHeightStore>,
    pub final_vote_store: Arc<LmdbFinalVoteStore>,
    pub unchecked_store: Arc<LmdbUncheckedStore>,
    pub version_store: Arc<LmdbVersionStore>,
    logger: Arc<dyn Logger>,
}

impl LmdbStore {
    pub fn new(
        path: &Path,
        options: &EnvOptions,
        tracking_cfg: TxnTrackingConfig,
        block_processor_batch_max_time: Duration,
        logger: Arc<dyn Logger>,
        backup_before_upgrade: bool,
    ) -> anyhow::Result<Self> {
        let env = Arc::new(LmdbEnv::with_tracking(
            path,
            options,
            tracking_cfg,
            block_processor_batch_max_time,
            logger.clone(),
        )?);

        let store = Self {
            env: env.clone(),
            block_store: Arc::new(LmdbBlockStore::new(env.clone())),
            frontier_store: Arc::new(LmdbFrontierStore::new(env.clone())),
            account_store: Arc::new(LmdbAccountStore::new(env.clone())),
            pending_store: Arc::new(LmdbPendingStore::new(env.clone())),
            online_weight_store: Arc::new(LmdbOnlineWeightStore::new(env.clone())),
            pruned_store: Arc::new(LmdbPrunedStore::new(env.clone())),
            peer_store: Arc::new(LmdbPeerStore::new(env.clone())),
            confirmation_height_store: Arc::new(LmdbConfirmationHeightStore::new(env.clone())),
            final_vote_store: Arc::new(LmdbFinalVoteStore::new(env.clone())),
            unchecked_store: Arc::new(LmdbUncheckedStore::new(env.clone())),
            version_store: Arc::new(LmdbVersionStore::new(env.clone())),
            logger,
        };

        let mut is_fully_upgraded = false;
        let mut is_fresh_db = false;
        {
            let transaction = store.env.tx_begin_read();
            if store.version_store.open_db(&transaction, 0).is_ok() {
                is_fully_upgraded = store.version_store.get(&transaction) == STORE_VERSION_CURRENT;
                unsafe {
                    mdb_dbi_close(store.env.env(), store.version_store.db_handle());
                }
            } else {
                is_fresh_db = true;
            }
        }

        // Only open a write lock when upgrades are needed. This is because CLI commands
        // open inactive nodes which can otherwise be locked here if there is a long write
        // (can be a few minutes with the --fast_bootstrap flag for instance)
        if !is_fully_upgraded {
            if !is_fresh_db {
                store.logger.always_log("Upgrade in progress...");
                if backup_before_upgrade {
                    create_backup_file(&store.env, path, store.logger.as_ref())?;
                }
            }
            let vacuuming = {
                let transaction = store.env.tx_begin_write();
                store.open_databases(&transaction, MDB_CREATE)?;
                store.do_upgrades(&transaction)?
            };

            if vacuuming == Vacuuming::Needed {
                store.logger.always_log("Preparing vacuum...");
                match store.vacuum_after_upgrade (path, &options.config){
                    Ok(_) => store.logger.always_log("Vacuum succeeded."),
                    Err(_) => store.logger.always_log("Failed to vacuum. (Optional) Ensure enough disk space is available for a copy of the database and try to vacuum after shutting down the node"),
                }
            }
        } else {
            let transaction = store.env.tx_begin_read();
            store.open_databases(&transaction, 0)?;
        }

        Ok(store)
    }

    pub fn open_databases(&self, txn: &dyn Transaction, flags: u32) -> anyhow::Result<()> {
        self.block_store.open_db(txn, flags)?;
        self.frontier_store.open_db(txn, flags)?;
        self.account_store.open_db(txn, flags)?;
        self.pending_store.open_db(txn, flags)?;
        self.online_weight_store.open_db(txn, flags)?;
        self.pruned_store.open_db(txn, flags)?;
        self.peer_store.open_db(txn, flags)?;
        self.confirmation_height_store.open_db(txn, flags)?;
        self.final_vote_store.open_db(txn, flags)?;
        self.unchecked_store.open_db(txn, flags)?;
        self.version_store.open_db(txn, flags)
    }

    pub fn do_upgrades(&self, txn: &dyn WriteTransaction) -> anyhow::Result<Vacuuming> {
        let version = self.version_store.get(txn.as_transaction());
        match version {
            1..=20 => {
                self.logger.always_log(&format!("The version of the ledger ({}) is lower than the minimum ({}) which is supported for upgrades. Either upgrade to a v23 node first or delete the ledger.", version, STORE_VERSION_MINIMUM));
                Err(anyhow!("version too low"))
            }
            21 => {
                // most recent version
                Ok(Vacuuming::NotNeeded)
            }
            _ => {
                self.logger.always_log(&format!(
                    "The version of the ledger ({}) is too high for this node",
                    version
                ));
                Err(anyhow!("version too high"))
            }
        }
    }

    pub fn vacuum_after_upgrade(&self, path: &Path, config: &LmdbConfig) -> anyhow::Result<()> {
        // Vacuum the database. This is not a required step and may actually fail if there isn't enough storage space.
        let mut vacuum_path = path.to_owned();
        vacuum_path.pop();
        vacuum_path.push("vacuumed.ldb");

        match self.copy_db(&vacuum_path) {
            Ok(_) => {
                self.env.close();

                // Replace the ledger file with the vacuumed one
                std::fs::rename(&vacuum_path, path)?;

                // Set up the environment again
                let options = EnvOptions {
                    config: config.clone(),
                    use_no_mem_init: true,
                };
                self.env.init(path, &options)?;
                let transaction = self.env.tx_begin_read();
                self.open_databases(&transaction, 0)
            }
            Err(e) => {
                // The vacuum file can be in an inconsistent state if there wasn't enough space to create it
                let _ = std::fs::remove_file(&vacuum_path);
                Err(e)
            }
        }
    }

    pub fn rebuild_db(&self, txn: &dyn WriteTransaction) -> anyhow::Result<()> {
        let raw_txn = get_raw_lmdb_txn(txn.as_transaction());
        // Tables with uint256_union key
        let tables = [
            self.account_store.db_handle(),
            self.block_store.db_handle(),
            self.pruned_store.db_handle(),
            self.confirmation_height_store.db_handle(),
        ];
        for table in tables {
            let mut temp = 0;
            unsafe {
                mdb_dbi_open(raw_txn, "temp_table", MDB_CREATE, &mut temp);
            }
            // Copy all values to temporary table

            let mut i = LmdbRawIterator::new(raw_txn, table, &MdbVal::new(), true, 32);
            while i.key.mv_size > 0 {
                let s = unsafe { mdb_put(raw_txn, temp, &mut i.key, &mut i.value, MDB_APPEND) };
                ensure_success(s)?;
                i.next();
            }

            if unsafe { mdb_count(raw_txn, table) != mdb_count(raw_txn, temp) } {
                bail!("table count mismatch");
            }

            // Clear existing table
            unsafe { mdb_drop(raw_txn, table, 0) };
            // Put values from copy
            let mut i = LmdbRawIterator::new(raw_txn, temp, &MdbVal::new(), true, 32);
            while i.key.mv_size > 0 {
                let s = unsafe { mdb_put(raw_txn, table, &mut i.key, &mut i.value, MDB_APPEND) };
                ensure_success(s)?;
                i.next();
            }

            if unsafe { mdb_count(raw_txn, table) != mdb_count(raw_txn, temp) } {
                bail!("table count mismatch");
            }
            // Remove temporary table
            unsafe { mdb_drop(raw_txn, temp, 1) };
        }
        // Pending table
        {
            let mut temp = 0;
            unsafe {
                mdb_dbi_open(raw_txn, "temp_table", MDB_CREATE, &mut temp);
            }
            // Copy all values to temporary table
            let mut i = LmdbRawIterator::new(
                raw_txn,
                self.pending_store.db_handle(),
                &MdbVal::new(),
                true,
                PendingKey::serialized_size(),
            );
            while i.key.mv_size > 0 {
                let s = unsafe { mdb_put(raw_txn, temp, &mut i.key, &mut i.value, MDB_APPEND) };
                ensure_success(s)?;
                i.next();
            }

            if unsafe {
                mdb_count(raw_txn, self.pending_store.db_handle()) != mdb_count(raw_txn, temp)
            } {
                bail!("table count mismatch");
            }
            unsafe { mdb_drop(raw_txn, self.pending_store.db_handle(), 0) };
            // Put values from copy
            let mut i = LmdbRawIterator::new(
                raw_txn,
                temp,
                &MdbVal::new(),
                true,
                PendingKey::serialized_size(),
            );
            while i.key.mv_size > 0 {
                let s = unsafe {
                    mdb_put(
                        raw_txn,
                        self.pending_store.db_handle(),
                        &mut i.key,
                        &mut i.value,
                        MDB_APPEND,
                    )
                };
                ensure_success(s)?;
                i.next();
            }
            if unsafe {
                mdb_count(raw_txn, self.pending_store.db_handle()) != mdb_count(raw_txn, temp)
            } {
                bail!("table count mismatch");
            }

            unsafe { mdb_drop(raw_txn, temp, 1) };
        }
        Ok(())
    }

    pub fn serialize_memory_stats(&self, json: &mut dyn PropertyTreeWriter) -> anyhow::Result<()> {
        let mut stats = MdbStat::default();
        let status = unsafe { mdb_env_stat(self.env.env(), &mut stats) };
        ensure_success(status)?;
        json.put_u64("branch_pages", stats.ms_branch_pages as u64)?;
        json.put_u64("depth", stats.ms_depth as u64)?;
        json.put_u64("entries", stats.ms_entries as u64)?;
        json.put_u64("leaf_pages", stats.ms_leaf_pages as u64)?;
        json.put_u64("overflow_pages", stats.ms_overflow_pages as u64)?;
        json.put_u64("page_size", stats.ms_psize as u64)?;
        Ok(())
    }
}

impl Store for LmdbStore {
    fn copy_db(&self, destination: &Path) -> anyhow::Result<()> {
        let c_path = CString::new(destination.as_os_str().to_str().unwrap()).unwrap();
        let status = unsafe { mdb_env_copy2(self.env.env(), c_path.as_ptr(), MDB_CP_COMPACT) };
        ensure_success(status)
    }
}

/// Takes a filepath, appends '_backup_<timestamp>' to the end (but before any extension) and saves that file in the same directory
pub fn create_backup_file(
    env: &LmdbEnv,
    source_path: &Path,
    logger: &dyn Logger,
) -> anyhow::Result<()> {
    let extension = source_path
        .extension()
        .ok_or_else(|| anyhow!("no extension"))?
        .to_string_lossy();
    let file_name = source_path
        .file_name()
        .ok_or_else(|| anyhow!("no file name"))?
        .to_string_lossy();
    let file_stem = source_path
        .file_stem()
        .ok_or_else(|| anyhow!("no file stem"))?
        .to_string_lossy();
    let mut backup_path = source_path
        .parent()
        .ok_or_else(|| anyhow!("no parent path"))?
        .to_owned();
    let backup_filename = format!(
        "{}_backup_{}.{}",
        file_stem,
        seconds_since_epoch(),
        extension
    );
    backup_path.push(&backup_filename);

    let start_message = format!("Performing {} backup before database upgrade...", file_name);
    logger.always_log(&start_message);
    println!("{}", start_message);

    let backup_path_cstr = CString::new(
        backup_path
            .as_os_str()
            .to_str()
            .ok_or_else(|| anyhow!("invalid backup path"))?,
    )?;
    let status = unsafe { mdb_env_copy(env.env(), backup_path_cstr.as_ptr()) };
    if status != MDB_SUCCESS {
        let error_message = format!("{} backup failed", file_name);
        logger.always_log(&error_message);
        eprintln!("{}", error_message);
        Err(anyhow!(error_message))
    } else {
        let success_message = format!("Backup created: {}", backup_filename);
        logger.always_log(&success_message);
        println!("{}", success_message);
        Ok(())
    }
}
