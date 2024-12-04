use crate::{
    EnvOptions, LmdbAccountStore, LmdbBlockStore, LmdbConfirmationHeightStore, LmdbDatabase,
    LmdbEnv, LmdbFinalVoteStore, LmdbOnlineWeightStore, LmdbPeerStore, LmdbPendingStore,
    LmdbPrunedStore, LmdbReadTransaction, LmdbRepWeightStore, LmdbVersionStore,
    LmdbWriteTransaction, NullTransactionTracker, TransactionTracker, STORE_VERSION_CURRENT,
    STORE_VERSION_MINIMUM,
};
use lmdb::{DatabaseFlags, WriteFlags};
use lmdb_sys::{MDB_CP_COMPACT, MDB_SUCCESS};
use rsnano_core::utils::seconds_since_epoch;
use serde::{Deserialize, Serialize};
use std::{
    ffi::CString,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use tracing::{debug, error, info, warn};

#[derive(PartialEq, Eq)]
pub enum Vacuuming {
    Needed,
    NotNeeded,
}

pub struct LedgerCache {
    pub cemented_count: AtomicU64,
    pub block_count: AtomicU64,
    pub pruned_count: AtomicU64,
    pub account_count: AtomicU64,
}

impl LedgerCache {
    pub fn new() -> Self {
        Self {
            cemented_count: AtomicU64::new(0),
            block_count: AtomicU64::new(0),
            pruned_count: AtomicU64::new(0),
            account_count: AtomicU64::new(0),
        }
    }

    pub fn reset(&self) {
        self.cemented_count.store(0, Ordering::SeqCst);
        self.block_count.store(0, Ordering::SeqCst);
        self.pruned_count.store(0, Ordering::SeqCst);
        self.account_count.store(0, Ordering::SeqCst);
    }
}

pub struct LmdbStore {
    pub env: Arc<LmdbEnv>,
    pub cache: Arc<LedgerCache>,
    pub block: Arc<LmdbBlockStore>,
    pub account: Arc<LmdbAccountStore>,
    pub pending: Arc<LmdbPendingStore>,
    pub online_weight: Arc<LmdbOnlineWeightStore>,
    pub pruned: Arc<LmdbPrunedStore>,
    pub rep_weight: Arc<LmdbRepWeightStore>,
    pub peer: Arc<LmdbPeerStore>,
    pub confirmation_height: Arc<LmdbConfirmationHeightStore>,
    pub final_vote: Arc<LmdbFinalVoteStore>,
    pub version: Arc<LmdbVersionStore>,
}

pub struct LmdbStoreBuilder<'a> {
    path: &'a Path,
    options: Option<&'a EnvOptions>,
    tracker: Option<Arc<dyn TransactionTracker>>,
    backup_before_upgrade: bool,
}

impl<'a> LmdbStoreBuilder<'a> {
    fn new(path: &'a Path) -> Self {
        Self {
            path,
            options: None,
            tracker: None,
            backup_before_upgrade: false,
        }
    }

    pub fn options(mut self, options: &'a EnvOptions) -> Self {
        self.options = Some(options);
        self
    }

    pub fn txn_tracker(mut self, tracker: Arc<dyn TransactionTracker>) -> Self {
        self.tracker = Some(tracker);
        self
    }

    pub fn backup_before_upgrade(mut self, backup: bool) -> Self {
        self.backup_before_upgrade = backup;
        self
    }

    pub fn build(self) -> anyhow::Result<LmdbStore> {
        let default_options = Default::default();
        let options = self.options.unwrap_or(&default_options);

        let txn_tracker = self
            .tracker
            .unwrap_or_else(|| Arc::new(NullTransactionTracker::new()));

        LmdbStore::new(self.path, options, txn_tracker, self.backup_before_upgrade)
    }
}

impl LmdbStore {
    pub fn new_null() -> Self {
        Self::new_with_env(LmdbEnv::new_null()).unwrap()
    }

    pub fn open(path: &Path) -> LmdbStoreBuilder<'_> {
        LmdbStoreBuilder::new(path)
    }

    fn new(
        path: impl AsRef<Path>,
        options: &EnvOptions,
        txn_tracker: Arc<dyn TransactionTracker>,
        backup_before_upgrade: bool,
    ) -> anyhow::Result<Self> {
        let path = path.as_ref();
        upgrade_if_needed(path, backup_before_upgrade)?;

        let env = LmdbEnv::new_with_txn_tracker(path, options, txn_tracker)?;
        Self::new_with_env(env)
    }

    fn new_with_env(env: LmdbEnv) -> anyhow::Result<Self> {
        let env = Arc::new(env);
        Ok(Self {
            cache: Arc::new(LedgerCache::new()),
            block: Arc::new(LmdbBlockStore::new(env.clone())?),
            account: Arc::new(LmdbAccountStore::new(env.clone())?),
            pending: Arc::new(LmdbPendingStore::new(env.clone())?),
            online_weight: Arc::new(LmdbOnlineWeightStore::new(env.clone())?),
            pruned: Arc::new(LmdbPrunedStore::new(env.clone())?),
            rep_weight: Arc::new(LmdbRepWeightStore::new(env.clone())?),
            peer: Arc::new(LmdbPeerStore::new(env.clone())?),
            confirmation_height: Arc::new(LmdbConfirmationHeightStore::new(env.clone())?),
            final_vote: Arc::new(LmdbFinalVoteStore::new(env.clone())?),
            version: Arc::new(LmdbVersionStore::new(env.clone())?),
            env,
        })
    }

    pub fn copy_db(&self, destination: &Path) -> anyhow::Result<()> {
        copy_db(&self.env, destination)
    }

    pub fn rebuild_db(&self, txn: &mut LmdbWriteTransaction) -> anyhow::Result<()> {
        let tables = [
            self.account.database(),
            self.block.database(),
            self.pruned.database(),
            self.confirmation_height.database(),
            self.pending.database(),
        ];
        for table in tables {
            rebuild_table(&self.env, txn, table)?;
        }

        Ok(())
    }

    pub fn memory_stats(&self) -> anyhow::Result<MemoryStats> {
        let stats = self.env.environment.stat()?;
        Ok(MemoryStats {
            branch_pages: stats.branch_pages(),
            depth: stats.depth(),
            entries: stats.entries(),
            leaf_pages: stats.leaf_pages(),
            overflow_pages: stats.overflow_pages(),
            page_size: stats.page_size(),
        })
    }

    pub fn vendor(&self) -> String {
        // fake version! TODO: read version
        format!("lmdb-rkv {}.{}.{}", 0, 14, 0)
    }

    pub fn tx_begin_read(&self) -> LmdbReadTransaction {
        self.env.tx_begin_read()
    }

    pub fn tx_begin_write(&self) -> LmdbWriteTransaction {
        self.env.tx_begin_write()
    }
}

fn upgrade_if_needed(path: &Path, backup_before_upgrade: bool) -> Result<(), anyhow::Error> {
    let upgrade_info = LmdbVersionStore::check_upgrade(path)?;
    if upgrade_info.is_fully_upgraded {
        debug!("No database upgrade needed");
        return Ok(());
    }

    let env = Arc::new(LmdbEnv::new(path)?);
    if !upgrade_info.is_fresh_db && backup_before_upgrade {
        create_backup_file(&env)?;
    }

    info!("Upgrade in progress...");
    let vacuuming = do_upgrades(env.clone())?;
    info!("Upgrade done!");

    if vacuuming == Vacuuming::Needed {
        info!("Preparing vacuum...");
        match vacuum_after_upgrade (env, path){
                Ok(_) => info!("Vacuum succeeded."),
                Err(_) => warn!("Failed to vacuum. (Optional) Ensure enough disk space is available for a copy of the database and try to vacuum after shutting down the node"),
            }
    }
    Ok(())
}

fn rebuild_table(
    env: &LmdbEnv,
    rw_txn: &mut LmdbWriteTransaction,
    db: LmdbDatabase,
) -> anyhow::Result<()> {
    let temp = unsafe {
        rw_txn
            .rw_txn_mut()
            .create_db(Some("temp_table"), DatabaseFlags::empty())
    }?;
    copy_table(env, rw_txn, db, temp)?;
    crate::Transaction::refresh(rw_txn);
    rw_txn.clear_db(db)?;
    copy_table(env, rw_txn, temp, db)?;
    unsafe { rw_txn.rw_txn_mut().drop_db(temp) }?;
    crate::Transaction::refresh(rw_txn);
    Ok(())
}

fn copy_table(
    env: &LmdbEnv,
    rw_txn: &mut LmdbWriteTransaction,
    source: LmdbDatabase,
    target: LmdbDatabase,
) -> anyhow::Result<()> {
    let ro_txn = env.tx_begin_read();
    {
        let mut cursor = ro_txn.txn().open_ro_cursor(source)?;
        for x in cursor.iter_start() {
            let (k, v) = x?;
            rw_txn.put(target, k, v, WriteFlags::APPEND)?;
        }
    }
    if ro_txn.txn().count(source) != rw_txn.rw_txn_mut().count(target) {
        bail!("table count mismatch");
    }
    Ok(())
}

fn do_upgrades(env: Arc<LmdbEnv>) -> anyhow::Result<Vacuuming> {
    let version_store = LmdbVersionStore::new(env.clone())?;
    let mut txn = env.tx_begin_write();

    let version = match version_store.get(&txn) {
        Some(v) => v,
        None => {
            let new_version = STORE_VERSION_MINIMUM;
            info!("Setting db version to {}", new_version);
            version_store.put(&mut txn, new_version);
            new_version
        }
    };

    if version < STORE_VERSION_MINIMUM {
        error!("The version of the ledger ({}) is lower than the minimum ({}) which is supported for upgrades. Either upgrade to a v24 node first or delete the ledger.", version, STORE_VERSION_MINIMUM);
        bail!("version too low");
    }

    if version > STORE_VERSION_CURRENT {
        error!(
            "The version of the ledger ({}) is too high for this node",
            version
        );
        bail!("version too high");
    }

    // most recent version
    Ok(Vacuuming::NotNeeded)
}

fn vacuum_after_upgrade(env: Arc<LmdbEnv>, path: &Path) -> anyhow::Result<()> {
    // Vacuum the database. This is not a required step and may actually fail if there isn't enough storage space.
    let mut vacuum_path = path.to_owned();
    vacuum_path.pop();
    vacuum_path.push("vacuumed.ldb");

    match copy_db(&env, &vacuum_path) {
        Ok(_) => {
            //todo don't use Arc here! Env must be dropped!
            drop(env);

            // Replace the ledger file with the vacuumed one
            std::fs::rename(&vacuum_path, path)?;
            Ok(())
        }
        Err(e) => {
            // The vacuum file can be in an inconsistent state if there wasn't enough space to create it
            let _ = std::fs::remove_file(&vacuum_path);
            Err(e)
        }
    }
}
fn copy_db(env: &LmdbEnv, destination: &Path) -> anyhow::Result<()> {
    let c_path = CString::new(destination.as_os_str().to_str().unwrap()).unwrap();
    let status =
        unsafe { lmdb_sys::mdb_env_copy2(env.environment.env(), c_path.as_ptr(), MDB_CP_COMPACT) };
    ensure_success(status)
}

fn ensure_success(status: i32) -> Result<(), anyhow::Error> {
    if status == MDB_SUCCESS {
        Ok(())
    } else {
        Err(anyhow!("lmdb returned status code {}", status))
    }
}

#[derive(Serialize, Deserialize)]
pub struct MemoryStats {
    pub branch_pages: usize,
    pub depth: u32,
    pub entries: usize,
    pub leaf_pages: usize,
    pub overflow_pages: usize,
    pub page_size: u32,
}

/// Takes a filepath, appends '_backup_<timestamp>' to the end (but before any extension) and saves that file in the same directory
pub fn create_backup_file(env: &LmdbEnv) -> anyhow::Result<()> {
    let source_path = env.file_path()?;
    let backup_path = backup_file_path(&source_path)?;

    info!(
        "Performing {:?} backup before database upgrade...",
        source_path
    );

    let backup_path_cstr = CString::new(
        backup_path
            .as_os_str()
            .to_str()
            .ok_or_else(|| anyhow!("invalid backup path"))?,
    )?;
    let status =
        unsafe { lmdb_sys::mdb_env_copy(env.environment.env(), backup_path_cstr.as_ptr()) };
    if status != MDB_SUCCESS {
        error!("{:?} backup failed", source_path);
        Err(anyhow!("backup failed"))
    } else {
        info!("Backup created: {:?}", backup_path);
        Ok(())
    }
}

fn backup_file_path(source_path: &Path) -> anyhow::Result<PathBuf> {
    let extension = source_path
        .extension()
        .ok_or_else(|| anyhow!("no extension"))?
        .to_str()
        .ok_or_else(|| anyhow!("invalid extension"))?;

    let mut backup_path = source_path
        .parent()
        .ok_or_else(|| anyhow!("no parent path"))?
        .to_owned();

    let file_stem = source_path
        .file_stem()
        .ok_or_else(|| anyhow!("no file stem"))?
        .to_str()
        .ok_or_else(|| anyhow!("invalid file stem"))?;

    let backup_filename = format!(
        "{}_backup_{}.{}",
        file_stem,
        seconds_since_epoch(),
        extension
    );
    backup_path.push(&backup_filename);
    Ok(backup_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TestDbFile;

    #[test]
    fn create_store() -> anyhow::Result<()> {
        let file = TestDbFile::random();
        let _ = LmdbStore::open(&file.path).build()?;
        Ok(())
    }

    #[test]
    fn version_too_high_for_upgrade() -> anyhow::Result<()> {
        let file = TestDbFile::random();
        set_store_version(&file, i32::MAX)?;
        assert_upgrade_fails(&file.path, "version too high");
        Ok(())
    }

    #[test]
    fn version_too_low_for_upgrade() -> anyhow::Result<()> {
        let file = TestDbFile::random();
        set_store_version(&file, STORE_VERSION_MINIMUM - 1)?;
        assert_upgrade_fails(&file.path, "version too low");
        Ok(())
    }

    #[test]
    fn writes_db_version_for_new_store() {
        let file = TestDbFile::random();
        let store = LmdbStore::open(&file.path).build().unwrap();
        let txn = store.tx_begin_read();
        assert_eq!(store.version.get(&txn), Some(STORE_VERSION_MINIMUM));
    }

    fn assert_upgrade_fails(path: &Path, error_msg: &str) {
        match LmdbStore::open(path).build() {
            Ok(_) => panic!("store should not be created!"),
            Err(e) => {
                assert_eq!(e.to_string(), error_msg);
            }
        }
    }

    fn set_store_version(file: &TestDbFile, current_version: i32) -> Result<(), anyhow::Error> {
        let env = Arc::new(LmdbEnv::new(&file.path)?);
        let version_store = LmdbVersionStore::new(env.clone())?;
        let mut txn = env.tx_begin_write();
        version_store.put(&mut txn, current_version);
        Ok(())
    }
}
