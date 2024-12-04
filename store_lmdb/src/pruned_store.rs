use crate::{
    BinaryDbIterator, LmdbDatabase, LmdbEnv, LmdbIteratorImpl, LmdbWriteTransaction, Transaction,
    PRUNED_TEST_DATABASE,
};
use lmdb::{DatabaseFlags, WriteFlags};
use rand::{thread_rng, Rng};
use rsnano_core::{BlockHash, NoValue};
use rsnano_nullable_lmdb::ConfiguredDatabase;
use std::sync::Arc;

pub type PrunedIterator<'txn> = BinaryDbIterator<'txn, BlockHash, NoValue>;

pub struct LmdbPrunedStore {
    database: LmdbDatabase,
}

impl LmdbPrunedStore {
    pub fn new(env: Arc<LmdbEnv>) -> anyhow::Result<Self> {
        let database = env
            .environment
            .create_db(Some("pruned"), DatabaseFlags::empty())?;
        Ok(Self { database })
    }

    pub fn database(&self) -> LmdbDatabase {
        self.database
    }

    pub fn put(&self, txn: &mut LmdbWriteTransaction, hash: &BlockHash) {
        txn.put(self.database, hash.as_bytes(), &[0; 0], WriteFlags::empty())
            .unwrap();
    }

    pub fn del(&self, txn: &mut LmdbWriteTransaction, hash: &BlockHash) {
        txn.delete(self.database, hash.as_bytes(), None).unwrap();
    }

    pub fn exists(&self, txn: &dyn Transaction, hash: &BlockHash) -> bool {
        txn.exists(self.database, hash.as_bytes())
    }

    pub fn begin<'txn>(&self, txn: &'txn dyn Transaction) -> PrunedIterator<'txn> {
        LmdbIteratorImpl::new_iterator(txn, self.database, None, true)
    }

    pub fn begin_at_hash<'txn>(
        &self,
        txn: &'txn dyn Transaction,
        hash: &BlockHash,
    ) -> PrunedIterator<'txn> {
        LmdbIteratorImpl::new_iterator(txn, self.database, Some(hash.as_bytes()), true)
    }

    pub fn random(&self, txn: &dyn Transaction) -> Option<BlockHash> {
        let random_hash = BlockHash::from_bytes(thread_rng().gen());
        let mut existing = self.begin_at_hash(txn, &random_hash);
        if existing.is_end() {
            existing = self.begin(txn);
        }

        existing.current().map(|(k, _)| *k)
    }

    pub fn count(&self, txn: &dyn Transaction) -> u64 {
        txn.count(self.database)
    }

    pub fn clear(&self, txn: &mut LmdbWriteTransaction) {
        txn.clear_db(self.database).unwrap();
    }

    pub fn end(&self) -> PrunedIterator<'static> {
        LmdbIteratorImpl::null_iterator()
    }
}

pub struct ConfiguredPrunedDatabaseBuilder {
    database: ConfiguredDatabase,
}

impl ConfiguredPrunedDatabaseBuilder {
    pub fn new() -> Self {
        Self {
            database: ConfiguredDatabase::new(PRUNED_TEST_DATABASE, "pruned"),
        }
    }

    pub fn pruned(mut self, hash: &BlockHash) -> Self {
        self.database
            .entries
            .insert(hash.as_bytes().to_vec(), Vec::new());
        self
    }

    pub fn build(self) -> ConfiguredDatabase {
        self.database
    }

    pub fn create(hashes: Vec<BlockHash>) -> ConfiguredDatabase {
        let mut builder = Self::new();
        for hash in hashes {
            builder = builder.pruned(&hash);
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
        store: LmdbPrunedStore,
    }

    impl Fixture {
        pub fn new() -> Self {
            Self::with_stored_data(Vec::new())
        }

        pub fn with_stored_data(entries: Vec<BlockHash>) -> Self {
            let env = LmdbEnv::new_null_with()
                .configured_database(ConfiguredPrunedDatabaseBuilder::create(entries))
                .build();
            let env = Arc::new(env);
            Self {
                env: env.clone(),
                store: LmdbPrunedStore::new(env).unwrap(),
            }
        }
    }

    #[test]
    fn empty_store() {
        let fixture = Fixture::new();
        let txn = fixture.env.tx_begin_read();
        let store = &fixture.store;

        assert_eq!(store.count(&txn), 0);
        assert_eq!(store.exists(&txn, &BlockHash::from(1)), false);
        assert_eq!(store.begin(&txn).is_end(), true);
        assert!(store.random(&txn).is_none());
    }

    #[test]
    fn add_pruned_info() {
        let fixture = Fixture::new();
        let mut txn = fixture.env.tx_begin_write();
        let put_tracker = txn.track_puts();
        let hash = BlockHash::from(1);

        fixture.store.put(&mut txn, &hash);

        assert_eq!(
            put_tracker.output(),
            vec![PutEvent {
                database: PRUNED_TEST_DATABASE.into(),
                key: hash.as_bytes().to_vec(),
                value: Vec::new(),
                flags: WriteFlags::empty()
            }]
        );
    }

    #[test]
    fn count() {
        let fixture = Fixture::with_stored_data(vec![BlockHash::from(1), BlockHash::from(2)]);
        let txn = fixture.env.tx_begin_read();

        assert_eq!(fixture.store.count(&txn), 2);
        assert_eq!(fixture.store.exists(&txn, &BlockHash::from(1)), true);
        assert_eq!(fixture.store.exists(&txn, &BlockHash::from(3)), false);
    }

    #[test]
    fn iterate() {
        let fixture = Fixture::with_stored_data(vec![BlockHash::from(1), BlockHash::from(2)]);
        let txn = fixture.env.tx_begin_read();

        assert_eq!(
            fixture.store.begin(&txn).current(),
            Some((&BlockHash::from(1), &NoValue {}))
        );
        assert_eq!(
            fixture
                .store
                .begin_at_hash(&txn, &BlockHash::from(2))
                .current(),
            Some((&BlockHash::from(2), &NoValue {}))
        );
    }

    #[test]
    fn delete() {
        let fixture = Fixture::new();
        let mut txn = fixture.env.tx_begin_write();
        let delete_tracker = txn.track_deletions();
        let hash = BlockHash::from(1);

        fixture.store.del(&mut txn, &hash);

        assert_eq!(
            delete_tracker.output(),
            vec![DeleteEvent {
                database: PRUNED_TEST_DATABASE.into(),
                key: hash.as_bytes().to_vec()
            }]
        )
    }

    #[test]
    fn pruned_random() {
        let fixture = Fixture::with_stored_data(vec![BlockHash::from(42)]);
        let txn = fixture.env.tx_begin_read();
        let random_hash = fixture.store.random(&txn);
        assert_eq!(random_hash, Some(BlockHash::from(42)));
    }
}
