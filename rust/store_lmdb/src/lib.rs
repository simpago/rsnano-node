#[macro_use]
extern crate num_derive;

#[macro_use]
extern crate anyhow;

mod account_store;
mod block_store;
mod confirmation_height_store;
mod fan;
mod final_vote_store;
mod iterator;
mod lmdb_config;
mod lmdb_env;
mod online_weight_store;
mod peer_store;
mod pending_store;
mod pruned_store;
mod rep_weight_store;
mod store;
mod version_store;
mod wallet_store;

pub use account_store::{ConfiguredAccountDatabaseBuilder, LmdbAccountStore};
pub use block_store::{ConfiguredBlockDatabaseBuilder, LmdbBlockStore};
pub use confirmation_height_store::*;
pub use fan::Fan;
pub use final_vote_store::LmdbFinalVoteStore;
pub use iterator::{BinaryDbIterator, LmdbIteratorImpl};
pub use lmdb_config::{LmdbConfig, SyncStrategy};
pub use lmdb_env::*;
pub use online_weight_store::LmdbOnlineWeightStore;
pub use peer_store::*;
pub use pending_store::{ConfiguredPendingDatabaseBuilder, LmdbPendingStore};
pub use pruned_store::{ConfiguredPrunedDatabaseBuilder, LmdbPrunedStore};
pub use rep_weight_store::*;
use rsnano_nullable_lmdb::{
    InactiveTransaction, LmdbDatabase, LmdbEnvironment, RoCursor, RoTransaction, RwTransaction,
};
pub use store::{create_backup_file, LedgerCache, LmdbStore};
pub use version_store::LmdbVersionStore;
pub use wallet_store::{Fans, KeyType, LmdbWalletStore, WalletValue};

use primitive_types::U256;
use rsnano_core::utils::{get_cpu_count, PropertyTree};
use std::{
    any::Any,
    cmp::{max, min},
    mem,
    sync::Arc,
    time::{Duration, Instant},
};

#[cfg(feature = "output_tracking")]
use rsnano_output_tracker::{OutputListener, OutputTracker};
#[cfg(feature = "output_tracking")]
use std::rc::Rc;

pub trait Transaction {
    fn as_any(&self) -> &dyn Any;
    fn refresh(&mut self);
    fn refresh_if_needed(&mut self);
    fn is_refresh_needed(&self) -> bool;
    fn get(&self, database: LmdbDatabase, key: &[u8]) -> lmdb::Result<&[u8]>;
    fn exists(&self, db: LmdbDatabase, key: &[u8]) -> bool {
        match self.get(db, key) {
            Ok(_) => true,
            Err(lmdb::Error::NotFound) => false,
            Err(e) => panic!("exists failed: {:?}", e),
        }
    }
    fn open_ro_cursor(&self, database: LmdbDatabase) -> lmdb::Result<RoCursor>;
    fn count(&self, database: LmdbDatabase) -> u64;
}

pub trait TransactionTracker: Send + Sync {
    fn txn_start(&self, txn_id: u64, is_write: bool);
    fn txn_end(&self, txn_id: u64, is_write: bool);
    fn serialize_json(
        &self,
        json: &mut dyn PropertyTree,
        min_read_time: Duration,
        min_write_time: Duration,
    ) -> anyhow::Result<()>;
}

pub struct NullTransactionTracker {}

impl NullTransactionTracker {
    pub fn new() -> Self {
        Self {}
    }
}

impl TransactionTracker for NullTransactionTracker {
    fn txn_start(&self, _txn_id: u64, _is_write: bool) {}

    fn txn_end(&self, _txn_id: u64, _is_write: bool) {}

    fn serialize_json(
        &self,
        _json: &mut dyn PropertyTree,
        _min_read_time: Duration,
        _min_write_time: Duration,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

enum RoTxnState {
    Inactive(InactiveTransaction),
    Active(RoTransaction),
    Transitioning,
}

pub struct LmdbReadTransaction {
    txn_id: u64,
    callbacks: Arc<dyn TransactionTracker>,
    txn: RoTxnState,
    start: Instant,
}

impl LmdbReadTransaction {
    pub fn new(
        txn_id: u64,
        env: &LmdbEnvironment,
        callbacks: Arc<dyn TransactionTracker>,
    ) -> lmdb::Result<Self> {
        let txn = env.begin_ro_txn()?;
        callbacks.txn_start(txn_id, false);

        Ok(Self {
            txn_id,
            callbacks,
            txn: RoTxnState::Active(txn),
            start: Instant::now(),
        })
    }

    pub fn txn(&self) -> &RoTransaction {
        match &self.txn {
            RoTxnState::Active(t) => t,
            _ => panic!("LMDB read transaction not active"),
        }
    }

    pub fn reset(&mut self) {
        let t = mem::replace(&mut self.txn, RoTxnState::Transitioning);
        self.txn = match t {
            RoTxnState::Active(t) => RoTxnState::Inactive(t.reset()),
            RoTxnState::Inactive(_) => panic!("Cannot reset inactive transaction"),
            RoTxnState::Transitioning => unreachable!(),
        };
        self.callbacks.txn_end(self.txn_id, false);
    }

    pub fn renew(&mut self) {
        let t = mem::replace(&mut self.txn, RoTxnState::Transitioning);
        self.txn = match t {
            RoTxnState::Active(_) => panic!("Cannot renew active transaction"),
            RoTxnState::Inactive(t) => RoTxnState::Active(t.renew().unwrap()),
            RoTxnState::Transitioning => unreachable!(),
        };
        self.callbacks.txn_start(self.txn_id, false);
        self.start = Instant::now();
    }
}

impl Drop for LmdbReadTransaction {
    fn drop(&mut self) {
        let t = mem::replace(&mut self.txn, RoTxnState::Transitioning);
        // This uses commit rather than abort, as it is needed when opening databases with a read only transaction
        if let RoTxnState::Active(t) = t {
            t.commit().unwrap()
        }
        self.callbacks.txn_end(self.txn_id, false);
    }
}

impl Transaction for LmdbReadTransaction {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn refresh(&mut self) {
        self.reset();
        self.renew();
    }

    fn is_refresh_needed(&self) -> bool {
        self.start.elapsed() > Duration::from_millis(500)
    }

    fn refresh_if_needed(&mut self) {
        if self.is_refresh_needed() {
            self.refresh();
        }
    }

    fn get(&self, database: LmdbDatabase, key: &[u8]) -> lmdb::Result<&[u8]> {
        self.txn().get(database, key)
    }

    fn open_ro_cursor(&self, database: LmdbDatabase) -> lmdb::Result<RoCursor> {
        self.txn().open_ro_cursor(database)
    }

    fn count(&self, database: LmdbDatabase) -> u64 {
        self.txn().count(database)
    }
}

enum RwTxnState {
    Inactive,
    Active(RwTransaction),
    Transitioning,
}

#[cfg(feature = "output_tracking")]
#[derive(Clone, Debug, PartialEq)]
pub struct PutEvent {
    database: LmdbDatabase,
    key: Vec<u8>,
    value: Vec<u8>,
    flags: lmdb::WriteFlags,
}

#[cfg(feature = "output_tracking")]
#[derive(Clone, Debug, PartialEq)]
pub struct DeleteEvent {
    database: LmdbDatabase,
    key: Vec<u8>,
}

pub struct LmdbWriteTransaction {
    env: &'static LmdbEnvironment,
    txn_id: u64,
    callbacks: Arc<dyn TransactionTracker>,
    txn: RwTxnState,
    #[cfg(feature = "output_tracking")]
    put_listener: OutputListener<PutEvent>,
    #[cfg(feature = "output_tracking")]
    delete_listener: OutputListener<DeleteEvent>,
    #[cfg(feature = "output_tracking")]
    clear_listener: OutputListener<LmdbDatabase>,
    start: Instant,
}

impl LmdbWriteTransaction {
    pub fn new<'a>(
        txn_id: u64,
        env: &'a LmdbEnvironment,
        callbacks: Arc<dyn TransactionTracker>,
    ) -> lmdb::Result<Self> {
        let env =
            unsafe { std::mem::transmute::<&'a LmdbEnvironment, &'static LmdbEnvironment>(env) };
        let mut tx = Self {
            env,
            txn_id,
            callbacks,
            txn: RwTxnState::Inactive,
            #[cfg(feature = "output_tracking")]
            put_listener: OutputListener::new(),
            #[cfg(feature = "output_tracking")]
            delete_listener: OutputListener::new(),
            #[cfg(feature = "output_tracking")]
            clear_listener: OutputListener::new(),
            start: Instant::now(),
        };
        tx.renew();
        Ok(tx)
    }

    pub fn rw_txn(&self) -> &RwTransaction {
        match &self.txn {
            RwTxnState::Active(t) => t,
            _ => panic!("txn not active"),
        }
    }

    pub fn rw_txn_mut(&mut self) -> &mut RwTransaction {
        match &mut self.txn {
            RwTxnState::Active(t) => t,
            _ => panic!("txn not active"),
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    pub fn renew(&mut self) {
        let t = mem::replace(&mut self.txn, RwTxnState::Transitioning);
        self.txn = match t {
            RwTxnState::Active(_) => panic!("Cannot renew active RwTransaction"),
            RwTxnState::Inactive => RwTxnState::Active(self.env.begin_rw_txn().unwrap()),
            RwTxnState::Transitioning => unreachable!(),
        };
        self.callbacks.txn_start(self.txn_id, true);
        self.start = Instant::now();
    }

    pub fn commit(&mut self) {
        let t = mem::replace(&mut self.txn, RwTxnState::Transitioning);
        match t {
            RwTxnState::Inactive => {}
            RwTxnState::Active(t) => {
                t.commit().unwrap();
                self.callbacks.txn_end(self.txn_id, true);
            }
            RwTxnState::Transitioning => unreachable!(),
        };
        self.txn = RwTxnState::Inactive;
    }

    #[cfg(feature = "output_tracking")]
    pub fn track_puts(&self) -> Rc<OutputTracker<PutEvent>> {
        self.put_listener.track()
    }

    #[cfg(feature = "output_tracking")]
    pub fn track_deletions(&self) -> Rc<OutputTracker<DeleteEvent>> {
        self.delete_listener.track()
    }

    #[cfg(feature = "output_tracking")]
    pub fn track_clears(&self) -> Rc<OutputTracker<LmdbDatabase>> {
        self.clear_listener.track()
    }

    pub unsafe fn create_db(
        &mut self,
        name: Option<&str>,
        flags: lmdb::DatabaseFlags,
    ) -> lmdb::Result<LmdbDatabase> {
        self.rw_txn().create_db(name, flags)
    }

    pub fn put(
        &mut self,
        database: LmdbDatabase,
        key: &[u8],
        value: &[u8],
        flags: lmdb::WriteFlags,
    ) -> lmdb::Result<()> {
        #[cfg(feature = "output_tracking")]
        self.put_listener.emit(PutEvent {
            database,
            key: key.to_vec(),
            value: value.to_vec(),
            flags,
        });
        self.rw_txn_mut().put(database, key, value, flags)
    }

    pub fn delete(
        &mut self,
        database: LmdbDatabase,
        key: &[u8],
        flags: Option<&[u8]>,
    ) -> lmdb::Result<()> {
        #[cfg(feature = "output_tracking")]
        self.delete_listener.emit(DeleteEvent {
            database,
            key: key.to_vec(),
        });
        self.rw_txn_mut().del(database, key, flags)
    }

    pub fn clear_db(&mut self, database: LmdbDatabase) -> lmdb::Result<()> {
        #[cfg(feature = "output_tracking")]
        self.clear_listener.emit(database);
        self.rw_txn_mut().clear_db(database)
    }
}

impl Drop for LmdbWriteTransaction {
    fn drop(&mut self) {
        self.commit();
    }
}

impl Transaction for LmdbWriteTransaction {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn refresh(&mut self) {
        self.commit();
        self.renew();
    }

    fn get(&self, database: LmdbDatabase, key: &[u8]) -> lmdb::Result<&[u8]> {
        self.rw_txn().get(database, key)
    }

    fn open_ro_cursor(&self, database: LmdbDatabase) -> lmdb::Result<RoCursor> {
        self.rw_txn().open_ro_cursor(database)
    }

    fn count(&self, database: LmdbDatabase) -> u64 {
        self.rw_txn().count(database)
    }

    fn is_refresh_needed(&self) -> bool {
        self.start.elapsed() > Duration::from_millis(500)
    }

    fn refresh_if_needed(&mut self) {
        if self.is_refresh_needed() {
            self.refresh();
        }
    }
}

pub fn parallel_traversal(action: &(impl Fn(U256, U256, bool) + Send + Sync)) {
    // Between 10 and 40 threads, scales well even in low power systems as long as actions are I/O bound
    let thread_count = max(10, min(40, 11 * get_cpu_count()));
    let split = U256::max_value() / thread_count;

    std::thread::scope(|s| {
        for thread in 0..thread_count {
            let start = split * thread;
            let end = split * (thread + 1);
            let is_last = thread == thread_count - 1;

            std::thread::Builder::new()
                .name("DB par traversl".to_owned())
                .spawn_scoped(s, move || {
                    action(start, end, is_last);
                })
                .unwrap();
        }
    });
}

pub const STORE_VERSION_MINIMUM: i32 = 24;
pub const STORE_VERSION_CURRENT: i32 = 24;

pub const BLOCK_TEST_DATABASE: LmdbDatabase = LmdbDatabase::new_null(1);
pub const FRONTIER_TEST_DATABASE: LmdbDatabase = LmdbDatabase::new_null(2);
pub const ACCOUNT_TEST_DATABASE: LmdbDatabase = LmdbDatabase::new_null(3);
pub const PENDING_TEST_DATABASE: LmdbDatabase = LmdbDatabase::new_null(4);
pub const PRUNED_TEST_DATABASE: LmdbDatabase = LmdbDatabase::new_null(5);
pub const REP_WEIGHT_TEST_DATABASE: LmdbDatabase = LmdbDatabase::new_null(6);
pub const CONFIRMATION_HEIGHT_TEST_DATABASE: LmdbDatabase = LmdbDatabase::new_null(7);
pub const PEERS_TEST_DATABASE: LmdbDatabase = LmdbDatabase::new_null(8);

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn tracks_deletes() {
        let env = LmdbEnv::new_null();
        let mut txn = env.tx_begin_write();
        let delete_tracker = txn.track_deletions();

        let database = LmdbDatabase::new_null(42);
        let key = vec![1, 2, 3];
        txn.delete(database, &key, None).unwrap();

        assert_eq!(delete_tracker.output(), vec![DeleteEvent { database, key }])
    }

    #[test]
    fn tracks_clears() {
        let env = LmdbEnv::new_null();
        let mut txn = env.tx_begin_write();
        let clear_tracker = txn.track_clears();

        let database = LmdbDatabase::new_null(42);
        txn.clear_db(database).unwrap();

        assert_eq!(clear_tracker.output(), vec![database])
    }
}
