use crate::{
    BinaryDbIterator, LmdbDatabase, LmdbEnv, LmdbIteratorImpl, LmdbWriteTransaction, Transaction,
};
use lmdb::{DatabaseFlags, WriteFlags};
use rsnano_core::Amount;
use std::sync::Arc;

pub type OnlineWeightIterator<'txn> = BinaryDbIterator<'txn, u64, Amount>;

pub struct LmdbOnlineWeightStore {
    _env: Arc<LmdbEnv>,
    database: LmdbDatabase,
}

impl LmdbOnlineWeightStore {
    pub fn new(env: Arc<LmdbEnv>) -> anyhow::Result<Self> {
        let database = env
            .environment
            .create_db(Some("online_weight"), DatabaseFlags::empty())?;
        Ok(Self {
            _env: env,
            database,
        })
    }

    pub fn database(&self) -> LmdbDatabase {
        self.database
    }

    pub fn put(&self, txn: &mut LmdbWriteTransaction, time: u64, amount: &Amount) {
        let time_bytes = time.to_be_bytes();
        let amount_bytes = amount.to_be_bytes();
        txn.put(
            self.database,
            &time_bytes,
            &amount_bytes,
            WriteFlags::empty(),
        )
        .unwrap();
    }

    pub fn del(&self, txn: &mut LmdbWriteTransaction, time: u64) {
        let time_bytes = time.to_be_bytes();
        txn.delete(self.database, &time_bytes, None).unwrap();
    }

    pub fn begin<'txn>(&self, txn: &'txn dyn Transaction) -> OnlineWeightIterator<'txn> {
        LmdbIteratorImpl::new_iterator(txn, self.database, None, true)
    }

    pub fn rbegin<'txn>(&self, txn: &'txn dyn Transaction) -> OnlineWeightIterator<'txn> {
        LmdbIteratorImpl::new_iterator(txn, self.database, None, false)
    }

    pub fn count(&self, txn: &dyn Transaction) -> u64 {
        txn.count(self.database)
    }

    pub fn clear(&self, txn: &mut LmdbWriteTransaction) {
        txn.clear_db(self.database).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DeleteEvent, PutEvent};

    struct Fixture {
        env: Arc<LmdbEnv>,
        store: LmdbOnlineWeightStore,
    }

    impl Fixture {
        fn new() -> Self {
            Self::with_stored_data(Vec::new())
        }

        fn with_stored_data(entries: Vec<(u64, Amount)>) -> Self {
            let mut env =
                LmdbEnv::new_null_with().database("online_weight", LmdbDatabase::new_null(42));

            for (key, value) in entries {
                env = env.entry(&key.to_be_bytes(), &value.to_be_bytes())
            }

            Self::with_env(env.build().build())
        }

        fn with_env(env: LmdbEnv) -> Self {
            let env = Arc::new(env);
            Self {
                env: env.clone(),
                store: LmdbOnlineWeightStore::new(env).unwrap(),
            }
        }
    }

    #[test]
    fn empty_store() {
        let fixture = Fixture::new();
        let txn = fixture.env.tx_begin_read();
        let store = &fixture.store;
        assert_eq!(store.count(&txn), 0);
        assert!(store.begin(&txn).is_end());
        assert!(store.rbegin(&txn).is_end());
    }

    #[test]
    fn count() {
        let fixture = Fixture::with_stored_data(vec![(1, Amount::raw(100)), (2, Amount::raw(200))]);
        let txn = fixture.env.tx_begin_read();

        let count = fixture.store.count(&txn);

        assert_eq!(count, 2);
    }

    #[test]
    fn add() {
        let fixture = Fixture::new();
        let mut txn = fixture.env.tx_begin_write();
        let put_tracker = txn.track_puts();

        let time = 1;
        let amount = Amount::raw(2);
        fixture.store.put(&mut txn, time, &amount);

        assert_eq!(
            put_tracker.output(),
            vec![PutEvent {
                database: LmdbDatabase::new_null(42),
                key: time.to_be_bytes().to_vec(),
                value: amount.to_be_bytes().to_vec(),
                flags: WriteFlags::empty(),
            }]
        );
    }

    #[test]
    fn iterate_ascending() {
        let fixture = Fixture::with_stored_data(vec![(1, Amount::raw(100)), (2, Amount::raw(200))]);
        let txn = fixture.env.tx_begin_read();

        let mut it = fixture.store.begin(&txn);
        assert_eq!(it.current(), Some((&1, &Amount::raw(100))));
        it.next();
        assert_eq!(it.current(), Some((&2, &Amount::raw(200))));
        it.next();
        assert_eq!(it.current(), None);
    }

    #[test]
    fn iterate_descending() {
        let fixture = Fixture::with_stored_data(vec![(1, Amount::raw(100)), (2, Amount::raw(200))]);
        let txn = fixture.env.tx_begin_read();

        let mut it = fixture.store.rbegin(&txn);
        assert_eq!(it.current(), Some((&2, &Amount::raw(200))));
        it.next();
        assert_eq!(it.current(), Some((&1, &Amount::raw(100))));
        it.next();
        assert_eq!(it.current(), None);
    }

    #[test]
    fn delete() {
        let fixture = Fixture::new();
        let mut txn = fixture.env.tx_begin_write();
        let delete_tracker = txn.track_deletions();

        let time = 1;
        fixture.store.del(&mut txn, time);

        assert_eq!(
            delete_tracker.output(),
            vec![DeleteEvent {
                database: LmdbDatabase::new_null(42),
                key: time.to_be_bytes().to_vec()
            }]
        );
    }
}
