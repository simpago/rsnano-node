use crate::nullable_lmdb::{ConfiguredDatabase, EnvironmentOptions, LmdbDatabase, LmdbEnvironment};
use crate::{
    LmdbConfig, LmdbReadTransaction, LmdbWriteTransaction, NullTransactionTracker, SyncStrategy,
    TransactionTracker,
};
use anyhow::bail;
use lmdb::EnvironmentFlags;
use lmdb_sys::MDB_SUCCESS;
use rsnano_core::utils::{memory_intensive_instrumentation, PropertyTree};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;
use std::{
    ffi::{c_char, CStr},
    fs::{create_dir_all, set_permissions, Permissions},
    os::unix::prelude::PermissionsExt,
    path::Path,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use tracing::debug;

#[derive(Default, Debug)]
pub struct EnvOptions {
    pub config: LmdbConfig,
    pub use_no_mem_init: bool,
}

pub struct NullLmdbEnvBuilder {
    databases: Vec<ConfiguredDatabase>,
}

impl NullLmdbEnvBuilder {
    pub fn database(self, name: impl Into<String>, dbi: LmdbDatabase) -> NullDatabaseBuilder {
        NullDatabaseBuilder {
            data: ConfiguredDatabase {
                dbi,
                db_name: name.into(),
                entries: BTreeMap::new(),
            },
            env_builder: self,
        }
    }

    pub fn configured_database(mut self, db: ConfiguredDatabase) -> Self {
        if self
            .databases
            .iter()
            .any(|x| x.dbi == db.dbi || x.db_name == db.db_name)
        {
            panic!(
                "trying to duplicated database for {} / {}",
                db.dbi.as_nulled(),
                db.db_name
            );
        }
        self.databases.push(db);
        self
    }

    pub fn build(self) -> LmdbEnv {
        let env = LmdbEnvironment::new_null_with(self.databases);
        LmdbEnv::new_with_env(env)
    }
}

pub struct NullDatabaseBuilder {
    env_builder: NullLmdbEnvBuilder,
    data: ConfiguredDatabase,
}

impl NullDatabaseBuilder {
    pub fn entry(mut self, key: &[u8], value: &[u8]) -> Self {
        self.data.entries.insert(key.to_vec(), value.to_vec());
        self
    }
    pub fn build(mut self) -> NullLmdbEnvBuilder {
        self.env_builder.databases.push(self.data);
        self.env_builder
    }
}

pub struct LmdbEnv {
    pub environment: LmdbEnvironment,
    next_txn_id: AtomicU64,
    txn_tracker: Arc<dyn TransactionTracker>,
    env_id: usize,
}

static ENV_COUNT: AtomicUsize = AtomicUsize::new(0);
static NEXT_ENV_ID: AtomicUsize = AtomicUsize::new(0);

impl LmdbEnv {
    pub fn new_null() -> Self {
        Self::new_with_env(LmdbEnvironment::new_null())
    }

    pub fn new_null_with() -> NullLmdbEnvBuilder {
        NullLmdbEnvBuilder {
            databases: Vec::new(),
        }
    }

    pub fn new(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        Self::new_with_options(path, &EnvOptions::default())
    }

    pub fn new_with_options(path: impl AsRef<Path>, options: &EnvOptions) -> anyhow::Result<Self> {
        let environment = Self::init(path.as_ref(), options)?;
        Ok(Self::new_with_env(environment))
    }

    pub fn new_with_env(env: LmdbEnvironment) -> Self {
        ENV_COUNT.fetch_add(1, Ordering::SeqCst);
        let env_id = NEXT_ENV_ID.fetch_add(1, Ordering::SeqCst);
        let alive = ENV_COUNT.fetch_add(1, Ordering::SeqCst) + 1;
        debug!(env_id, alive, "LMDB env created",);
        Self {
            environment: env,
            next_txn_id: AtomicU64::new(0),
            txn_tracker: Arc::new(NullTransactionTracker::new()),
            env_id,
        }
    }

    pub fn new_with_txn_tracker(
        path: &Path,
        options: &EnvOptions,
        txn_tracker: Arc<dyn TransactionTracker>,
    ) -> anyhow::Result<Self> {
        let env = Self {
            environment: Self::init(path, options)?,
            next_txn_id: AtomicU64::new(0),
            txn_tracker,
            env_id: NEXT_ENV_ID.fetch_add(1, Ordering::SeqCst),
        };
        let alive = ENV_COUNT.fetch_add(1, Ordering::SeqCst) + 1;
        debug!(env_id = env.env_id, alive, ?path, "LMDB env created",);
        Ok(env)
    }

    pub fn init(path: impl AsRef<Path>, options: &EnvOptions) -> anyhow::Result<LmdbEnvironment> {
        let path = path.as_ref();
        debug_assert!(
            path.extension() == Some(&OsStr::new("ldb")),
            "invalid filename extension for lmdb database file"
        );
        try_create_parent_dir(path)?;
        let mut map_size = options.config.map_size;
        let max_instrumented_map_size = 16 * 1024 * 1024;
        if memory_intensive_instrumentation() && map_size > max_instrumented_map_size {
            // In order to run LMDB under Valgrind, the maximum map size must be smaller than half your available RAM
            map_size = max_instrumented_map_size;
        }

        // It seems if there's ever more threads than mdb_env_set_maxreaders has read slots available, we get failures on transaction creation unless MDB_NOTLS is specified
        // This can happen if something like 256 io_threads are specified in the node config
        // MDB_NORDAHEAD will allow platforms that support it to load the DB in memory as needed.
        // MDB_NOMEMINIT prevents zeroing malloc'ed pages. Can provide improvement for non-sensitive data but may make memory checkers noisy (e.g valgrind).
        let mut environment_flags = EnvironmentFlags::NO_SUB_DIR
            | EnvironmentFlags::NO_TLS
            | EnvironmentFlags::NO_READAHEAD;
        if options.config.sync == SyncStrategy::NosyncSafe {
            environment_flags |= EnvironmentFlags::NO_META_SYNC;
        } else if options.config.sync == SyncStrategy::NosyncUnsafe {
            environment_flags |= EnvironmentFlags::NO_SYNC;
        } else if options.config.sync == SyncStrategy::NosyncUnsafeLargeMemory {
            environment_flags |= EnvironmentFlags::NO_SYNC
                | EnvironmentFlags::WRITE_MAP
                | EnvironmentFlags::MAP_ASYNC;
        }

        if !memory_intensive_instrumentation() && options.use_no_mem_init {
            environment_flags |= EnvironmentFlags::NO_MEM_INIT;
        }
        let env_options = EnvironmentOptions {
            max_dbs: options.config.max_databases,
            map_size,
            flags: environment_flags,
            path,
            file_mode: 0o600,
        };
        let env = LmdbEnvironment::new(env_options)?;
        Ok(env)
    }

    pub fn tx_begin_read(&self) -> LmdbReadTransaction {
        let txn_id = self.next_txn_id.fetch_add(1, Ordering::Relaxed);
        LmdbReadTransaction::new(txn_id, &self.environment, self.create_txn_callbacks())
            .expect("Could not create LMDB read-only transaction")
    }

    pub fn tx_begin_write(&self) -> LmdbWriteTransaction {
        // For IO threads, we do not want them to block on creating write transactions.
        debug_assert!(std::thread::current().name() != Some("I/O"));
        let txn_id = self.next_txn_id.fetch_add(1, Ordering::Relaxed);
        LmdbWriteTransaction::new(txn_id, &self.environment, self.create_txn_callbacks())
            .expect("Could not create LMDB read-write transaction")
    }

    pub fn file_path(&self) -> anyhow::Result<PathBuf> {
        let mut path: *const c_char = std::ptr::null();
        let status = unsafe { lmdb_sys::mdb_env_get_path(self.environment.env(), &mut path) };
        if status != MDB_SUCCESS {
            bail!("could not get env path");
        }
        let source_path: PathBuf = unsafe { CStr::from_ptr(path) }.to_str()?.into();
        Ok(source_path)
    }

    fn create_txn_callbacks(&self) -> Arc<dyn TransactionTracker> {
        Arc::clone(&self.txn_tracker)
    }

    pub fn serialize_txn_tracker(
        &self,
        json: &mut dyn PropertyTree,
        min_read_time: Duration,
        min_write_time: Duration,
    ) -> anyhow::Result<()> {
        self.txn_tracker
            .serialize_json(json, min_read_time, min_write_time)
    }
}

fn try_create_parent_dir(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        if parent != Path::new("") && !parent.is_dir() {
            create_dir_all(parent)?;
            set_permissions(parent, Permissions::from_mode(0o700))?;
        }
    }
    Ok(())
}

impl Drop for LmdbEnv {
    fn drop(&mut self) {
        let alive = ENV_COUNT.fetch_sub(1, Ordering::Relaxed) - 1;
        debug!(env_id = self.env_id, alive, "LMDB env dropped",);
        let _ = self.environment.sync(true);
    }
}

pub struct TestDbFile {
    pub path: PathBuf,
}

impl TestDbFile {
    fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: Path::new("/tmp").join(path),
        }
    }

    pub fn random() -> Self {
        Self::new(Self::temp_file_name())
    }

    fn temp_file_name() -> PathBuf {
        PathBuf::from(format!("{}.ldb", uuid::Uuid::new_v4().to_simple()))
    }

    fn lock_file_path(&self) -> PathBuf {
        let mut lock_file_path = self.path.parent().unwrap().to_owned();
        let mut fname = self.path.file_name().unwrap().to_os_string();
        fname.push("-lock");
        lock_file_path.push(fname);
        lock_file_path
    }
}

impl Drop for TestDbFile {
    fn drop(&mut self) {
        if self.path.exists() {
            std::fs::remove_file(&self.path).unwrap();
            let lock_file = self.lock_file_path();
            if lock_file.exists() {
                std::fs::remove_file(&lock_file).unwrap();
            }

            if let Some(parent) = self.path.parent() {
                if parent != Path::new("/tmp") {
                    std::fs::remove_dir(parent).unwrap();
                }
            }
        }
    }
}

pub struct TestLmdbEnv {
    env: Arc<LmdbEnv>,
    _file: TestDbFile,
}

impl TestLmdbEnv {
    pub fn new() -> Self {
        let file = TestDbFile::random();
        let env = Arc::new(LmdbEnv::new(&file.path).unwrap());
        Self { _file: file, env }
    }

    pub fn env(&self) -> Arc<LmdbEnv> {
        self.env.clone()
    }
}

impl Deref for TestLmdbEnv {
    type Target = LmdbEnv;

    fn deref(&self) -> &Self::Target {
        &self.env
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod rw_txn {
        use super::*;
        use crate::PutEvent;
        use lmdb::WriteFlags;

        #[test]
        fn can_track_puts() {
            let env = LmdbEnv::new_null();
            let mut txn = env.tx_begin_write();
            let tracker = txn.track_puts();

            let database = LmdbDatabase::new_null(42);
            let key = &[1, 2, 3];
            let value = &[4, 5, 6];
            let flags = WriteFlags::APPEND;
            txn.put(database, key, value, flags).unwrap();

            let puts = tracker.output();
            assert_eq!(
                puts,
                vec![PutEvent {
                    database,
                    key: key.to_vec(),
                    value: value.to_vec(),
                    flags
                }]
            )
        }
    }

    mod test_db_file {
        use super::*;

        #[test]
        fn tmp_test() {
            let path = Path::new("foo.tmp");
            assert_eq!(path.parent(), Some(Path::new("")));
            assert_eq!(Path::new(""), Path::new(""))
        }

        #[test]
        fn dont_panic_when_file_not_found() {
            let file = TestDbFile::new("does-not-exist.ldb");
            drop(file)
        }

        #[test]
        fn delete_file_when_dropped() {
            let file = TestDbFile::new("drop-test.ldb");
            let mut lock_file_path = file.path.parent().unwrap().to_owned();
            lock_file_path.push("drop-test.ldb-lock");
            std::fs::write(&file.path, "foo").unwrap();
            std::fs::write(&lock_file_path, "foo").unwrap();
            let path = file.path.clone();
            drop(file);
            assert_eq!(path.exists(), false, "db file was not deleted");
            assert_eq!(lock_file_path.exists(), false, "lock file was not deleted");
        }

        #[test]
        fn delete_dir_when_dropped() {
            let file = TestDbFile::new("drop-dir/db.ldb");
            std::fs::create_dir(file.path.parent().unwrap()).unwrap();
            std::fs::write(&file.path, "foo").unwrap();
            let path = file.path.clone();
            drop(file);
            assert_eq!(path.exists(), false);
            assert_eq!(path.parent().unwrap().exists(), false);
        }

        #[test]
        fn tmp_file_name() {
            let filename = TestDbFile::temp_file_name();
            assert_eq!(filename.extension().unwrap(), "ldb");
            assert_eq!(filename.file_stem().unwrap().len(), 32);
            assert_ne!(TestDbFile::temp_file_name(), TestDbFile::temp_file_name());
        }
    }
}
