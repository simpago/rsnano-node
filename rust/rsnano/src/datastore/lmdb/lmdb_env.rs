use super::{
    assert_success, mdb_env_close, mdb_env_create, mdb_env_sync, LmdbReadTransaction,
    LmdbWriteTransaction, MdbEnv, NullTxnCallbacks, TxnCallbacks, TxnTracker,
};
use crate::{
    datastore::lmdb::{
        mdb_env_open, mdb_env_set_mapsize, mdb_env_set_maxdbs, MDB_MAPASYNC, MDB_NOMEMINIT,
        MDB_NOMETASYNC, MDB_NORDAHEAD, MDB_NOSUBDIR, MDB_NOSYNC, MDB_NOTLS, MDB_WRITEMAP,
    },
    logger_mt::Logger,
    memory_intensive_instrumentation,
    utils::PropertyTreeWriter,
    LmdbConfig, SyncStrategy, TxnTrackingConfig,
};
use anyhow::Result;
use std::{
    fs::{create_dir_all, set_permissions, Permissions},
    os::unix::prelude::PermissionsExt,
    path::Path,
    ptr,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

pub struct EnvOptions {
    pub config: LmdbConfig,
    pub use_no_mem_init: bool,
}

pub struct LmdbEnv {
    environment: AtomicUsize,
    next_txn_id: AtomicU64,
    txn_tracker: Option<Arc<TxnTracker>>,
}

impl LmdbEnv {
    pub fn new(error: &mut bool, path: &Path, options: &EnvOptions) -> Self {
        let result = Self {
            environment: AtomicUsize::new(0),
            next_txn_id: AtomicU64::new(0),
            txn_tracker: None,
        };
        *error = result.init(path, options).is_err();
        result
    }

    pub fn with_tracking(
        path: &Path,
        options: &EnvOptions,
        tracking_cfg: TxnTrackingConfig,
        block_processor_batch_max_time: Duration,
        logger: Arc<dyn Logger>,
    ) -> Result<Self> {
        let txn_tracker = if tracking_cfg.enable {
            Some(Arc::new(TxnTracker::new(
                logger,
                tracking_cfg,
                block_processor_batch_max_time,
            )))
        } else {
            None
        };

        let result = Self {
            environment: AtomicUsize::new(0),
            next_txn_id: AtomicU64::new(0),
            txn_tracker,
        };
        result.init(path, options)?;
        Ok(result)
    }

    pub fn init(&self, path: &Path, options: &EnvOptions) -> Result<()> {
        let parent = path.parent().ok_or_else(|| anyhow!("no parent path"))?;
        create_dir_all(parent)?;
        let perms = Permissions::from_mode(0o700);
        let _ = set_permissions(parent, perms);
        let mut environment: *mut MdbEnv = ptr::null_mut();
        assert_success(unsafe { mdb_env_create(&mut environment) });
        self.environment
            .store(environment as usize, Ordering::SeqCst);
        assert_success(unsafe { mdb_env_set_maxdbs(self.env(), options.config.max_databases) });
        let mut map_size = options.config.map_size;
        let max_instrumented_map_size = 16 * 1024 * 1024;
        if memory_intensive_instrumentation() && map_size > max_instrumented_map_size {
            // In order to run LMDB under Valgrind, the maximum map size must be smaller than half your available RAM
            map_size = max_instrumented_map_size;
        }
        assert_success(unsafe { mdb_env_set_mapsize(self.env(), map_size) });
        // It seems if there's ever more threads than mdb_env_set_maxreaders has read slots available, we get failures on transaction creation unless MDB_NOTLS is specified
        // This can happen if something like 256 io_threads are specified in the node config
        // MDB_NORDAHEAD will allow platforms that support it to load the DB in memory as needed.
        // MDB_NOMEMINIT prevents zeroing malloc'ed pages. Can provide improvement for non-sensitive data but may make memory checkers noisy (e.g valgrind).
        let mut environment_flags = MDB_NOSUBDIR | MDB_NOTLS | MDB_NORDAHEAD;
        if options.config.sync == SyncStrategy::NosyncSafe {
            environment_flags |= MDB_NOMETASYNC;
        } else if options.config.sync == SyncStrategy::NosyncUnsafe {
            environment_flags |= MDB_NOSYNC;
        } else if options.config.sync == SyncStrategy::NosyncUnsafeLargeMemory {
            environment_flags |= MDB_NOSYNC | MDB_WRITEMAP | MDB_MAPASYNC;
        }

        if !memory_intensive_instrumentation() && options.use_no_mem_init {
            environment_flags |= MDB_NOMEMINIT;
        }

        assert_success(unsafe { mdb_env_open(self.env(), path, environment_flags, 0o600) });
        Ok(())
    }

    pub fn env(&self) -> *mut MdbEnv {
        self.environment.load(Ordering::SeqCst) as *mut MdbEnv
    }

    pub fn tx_begin_read(&self) -> LmdbReadTransaction {
        let txn_id = self.next_txn_id.fetch_add(1, Ordering::Relaxed);
        unsafe { LmdbReadTransaction::new(txn_id, self.env(), self.create_txn_callbacks()) }
    }

    pub fn tx_begin_write(&self) -> LmdbWriteTransaction {
        // For IO threads, we do not want them to block on creating write transactions.
        debug_assert!(std::thread::current().name() != Some("I/O"));
        let txn_id = self.next_txn_id.fetch_add(1, Ordering::Relaxed);
        unsafe { LmdbWriteTransaction::new(txn_id, self.env(), self.create_txn_callbacks()) }
    }

    fn create_txn_callbacks(&self) -> Arc<dyn TxnCallbacks> {
        match &self.txn_tracker {
            Some(tracker) => Arc::clone(tracker) as Arc<dyn TxnCallbacks>,
            None => Arc::new(NullTxnCallbacks::new()),
        }
    }

    pub fn close(&self) {
        if !self.env().is_null() {
            // Make sure the commits are flushed. This is a no-op unless MDB_NOSYNC is used.
            unsafe {
                mdb_env_sync(self.env(), true);
                mdb_env_close(self.env());
                self.environment.store(0, Ordering::SeqCst);
            }
        }
    }

    pub fn serialize_txn_tracker(
        &self,
        json: &mut dyn PropertyTreeWriter,
        min_read_time: Duration,
        min_write_time: Duration,
    ) -> anyhow::Result<()> {
        match &self.txn_tracker {
            Some(tracker) => tracker.serialize_json(json, min_read_time, min_write_time),
            None => Ok(()),
        }
    }
}

impl Drop for LmdbEnv {
    fn drop(&mut self) {
        self.close();
    }
}
