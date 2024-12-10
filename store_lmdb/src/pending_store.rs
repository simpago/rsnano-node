use crate::{
    iterator::LmdbRangeIterator, BinaryDbIterator, LmdbDatabase, LmdbEnv, LmdbIteratorImpl,
    LmdbWriteTransaction, Transaction, PENDING_TEST_DATABASE,
};
use lmdb::{DatabaseFlags, WriteFlags};
use rsnano_core::{
    utils::{BufferReader, Deserialize},
    Account, BlockHash, PendingInfo, PendingKey,
};
use rsnano_nullable_lmdb::ConfiguredDatabase;
#[cfg(feature = "output_tracking")]
use rsnano_output_tracker::{OutputListenerMt, OutputTrackerMt};
use std::{ops::RangeBounds, sync::Arc};

pub type PendingIterator<'txn> = BinaryDbIterator<'txn, PendingKey, PendingInfo>;

pub struct LmdbPendingStore {
    _env: Arc<LmdbEnv>,
    database: LmdbDatabase,
    #[cfg(feature = "output_tracking")]
    put_listener: OutputListenerMt<(PendingKey, PendingInfo)>,
    #[cfg(feature = "output_tracking")]
    delete_listener: OutputListenerMt<PendingKey>,
}

impl LmdbPendingStore {
    pub fn new(env: Arc<LmdbEnv>) -> anyhow::Result<Self> {
        let database = env
            .environment
            .create_db(Some("pending"), DatabaseFlags::empty())?;

        Ok(Self {
            _env: env,
            database,
            #[cfg(feature = "output_tracking")]
            put_listener: OutputListenerMt::new(),
            #[cfg(feature = "output_tracking")]
            delete_listener: OutputListenerMt::new(),
        })
    }

    pub fn database(&self) -> LmdbDatabase {
        self.database
    }

    #[cfg(feature = "output_tracking")]
    pub fn track_puts(&self) -> Arc<OutputTrackerMt<(PendingKey, PendingInfo)>> {
        self.put_listener.track()
    }

    #[cfg(feature = "output_tracking")]
    pub fn track_deletions(&self) -> Arc<OutputTrackerMt<PendingKey>> {
        self.delete_listener.track()
    }

    pub fn put(&self, txn: &mut LmdbWriteTransaction, key: &PendingKey, pending: &PendingInfo) {
        #[cfg(feature = "output_tracking")]
        self.put_listener.emit((key.clone(), pending.clone()));
        let key_bytes = key.to_bytes();
        let pending_bytes = pending.to_bytes();
        txn.put(
            self.database,
            &key_bytes,
            &pending_bytes,
            WriteFlags::empty(),
        )
        .unwrap();
    }

    pub fn del(&self, txn: &mut LmdbWriteTransaction, key: &PendingKey) {
        #[cfg(feature = "output_tracking")]
        self.delete_listener.emit(key.clone());
        let key_bytes = key.to_bytes();
        txn.delete(self.database, &key_bytes, None).unwrap();
    }

    pub fn get(&self, txn: &dyn Transaction, key: &PendingKey) -> Option<PendingInfo> {
        let key_bytes = key.to_bytes();
        match txn.get(self.database, &key_bytes) {
            Ok(bytes) => {
                let mut stream = BufferReader::new(bytes);
                PendingInfo::deserialize(&mut stream).ok()
            }
            Err(lmdb::Error::NotFound) => None,
            Err(e) => {
                panic!("Could not load pending info: {:?}", e);
            }
        }
    }

    pub fn iter_range<'txn>(
        &self,
        tx: &'txn dyn Transaction,
        range: impl RangeBounds<PendingKey> + 'static,
    ) -> impl Iterator<Item = (PendingKey, PendingInfo)> + 'txn {
        let cursor = tx.open_ro_cursor(self.database).unwrap();
        LmdbRangeIterator::new(cursor, range)
    }

    pub fn begin<'txn>(&self, txn: &'txn dyn Transaction) -> PendingIterator<'txn> {
        LmdbIteratorImpl::new_iterator(txn, self.database, None, true)
    }

    pub fn begin_at_key<'txn>(
        &self,
        txn: &'txn dyn Transaction,
        key: &PendingKey,
    ) -> PendingIterator<'txn> {
        let key_bytes = key.to_bytes();
        LmdbIteratorImpl::new_iterator(txn, self.database, Some(&key_bytes), true)
    }

    pub fn exists(&self, txn: &dyn Transaction, key: &PendingKey) -> bool {
        let iterator = self.begin_at_key(txn, key);
        iterator.current().map(|(k, _)| k == key).unwrap_or(false)
    }

    pub fn any(&self, txn: &dyn Transaction, account: &Account) -> bool {
        let key = PendingKey::new(*account, BlockHash::zero());
        let iterator = self.begin_at_key(txn, &key);
        iterator
            .current()
            .map(|(k, _)| k.receiving_account == *account)
            .unwrap_or(false)
    }

    pub fn end(&self) -> PendingIterator {
        LmdbIteratorImpl::null_iterator()
    }
}

pub struct ConfiguredPendingDatabaseBuilder {
    database: ConfiguredDatabase,
}

impl ConfiguredPendingDatabaseBuilder {
    pub fn new() -> Self {
        Self {
            database: ConfiguredDatabase::new(PENDING_TEST_DATABASE, "pending"),
        }
    }

    pub fn pending(mut self, key: &PendingKey, info: &PendingInfo) -> Self {
        self.database
            .entries
            .insert(key.to_bytes().to_vec(), info.to_bytes().to_vec());
        self
    }

    pub fn build(self) -> ConfiguredDatabase {
        self.database
    }

    pub fn create(frontiers: Vec<(PendingKey, PendingInfo)>) -> ConfiguredDatabase {
        let mut builder = Self::new();
        for (key, info) in frontiers {
            builder = builder.pending(&key, &info);
        }
        builder.build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DeleteEvent, PutEvent};

    struct Fixture {
        env: Arc<LmdbEnv>,
        store: LmdbPendingStore,
    }

    impl Fixture {
        pub fn new() -> Self {
            Self::with_stored_data(Vec::new())
        }

        pub fn with_stored_data(entries: Vec<(PendingKey, PendingInfo)>) -> Self {
            let env = LmdbEnv::new_null_with()
                .configured_database(ConfiguredPendingDatabaseBuilder::create(entries))
                .build();

            let env = Arc::new(env);
            Self {
                env: env.clone(),
                store: LmdbPendingStore::new(env).unwrap(),
            }
        }
    }

    #[test]
    fn not_found() {
        let fixture = Fixture::new();
        let txn = fixture.env.tx_begin_read();
        let result = fixture.store.get(&txn, &PendingKey::new_test_instance());
        assert!(result.is_none());
        assert_eq!(
            fixture.store.exists(&txn, &PendingKey::new_test_instance()),
            false
        );
    }

    #[test]
    fn load_pending_info() {
        let key = PendingKey::new_test_instance();
        let info = PendingInfo::new_test_instance();
        let fixture = Fixture::with_stored_data(vec![(key.clone(), info.clone())]);
        let txn = fixture.env.tx_begin_read();

        let result = fixture.store.get(&txn, &key);

        assert_eq!(result, Some(info));
        assert_eq!(fixture.store.exists(&txn, &key), true);
    }

    #[test]
    fn add_pending() {
        let fixture = Fixture::new();
        let mut txn = fixture.env.tx_begin_write();
        let put_tracker = txn.track_puts();
        let pending_key = PendingKey::new_test_instance();
        let pending = PendingInfo::new_test_instance();

        fixture.store.put(&mut txn, &pending_key, &pending);

        assert_eq!(
            put_tracker.output(),
            vec![PutEvent {
                database: PENDING_TEST_DATABASE.into(),
                key: pending_key.to_bytes().to_vec(),
                value: pending.to_bytes().to_vec(),
                flags: WriteFlags::empty()
            }]
        );
    }

    #[test]
    fn delete() {
        let fixture = Fixture::new();
        let mut txn = fixture.env.tx_begin_write();
        let delete_tracker = txn.track_deletions();
        let pending_key = PendingKey::new_test_instance();

        fixture.store.del(&mut txn, &pending_key);

        assert_eq!(
            delete_tracker.output(),
            vec![DeleteEvent {
                database: PENDING_TEST_DATABASE.into(),
                key: pending_key.to_bytes().to_vec()
            }]
        )
    }

    #[test]
    fn iter_empty() {
        let fixture = Fixture::new();
        let txn = fixture.env.tx_begin_read();
        assert!(fixture.store.begin(&txn).is_end());
    }

    #[test]
    fn iter() {
        let key = PendingKey::new_test_instance();
        let info = PendingInfo::new_test_instance();
        let fixture = Fixture::with_stored_data(vec![(key.clone(), info.clone())]);
        let txn = fixture.env.tx_begin_read();

        let mut it = fixture.store.begin(&txn);
        assert_eq!(it.is_end(), false);
        let (k, v) = it.current().unwrap();
        assert_eq!(k, &key);
        assert_eq!(v, &info);

        it.next();
        assert!(it.is_end());
    }

    #[test]
    fn tracks_puts() {
        let fixture = Fixture::new();
        let mut txn = fixture.env.tx_begin_write();
        let key = PendingKey::new_test_instance();
        let info = PendingInfo::new_test_instance();
        let put_tracker = fixture.store.track_puts();

        fixture.store.put(&mut txn, &key, &info);

        assert_eq!(put_tracker.output(), vec![(key, info)]);
    }

    #[test]
    fn tracks_deletions() {
        let fixture = Fixture::new();
        let mut txn = fixture.env.tx_begin_write();
        let key = PendingKey::new_test_instance();
        let delete_tracker = fixture.store.track_deletions();

        fixture.store.del(&mut txn, &key);

        assert_eq!(delete_tracker.output(), vec![key]);
    }
}
