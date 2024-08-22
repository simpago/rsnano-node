use crate::{
    LmdbDatabase, LmdbEnv, LmdbWriteTransaction, RoCursor, Transaction, REP_WEIGHT_TEST_DATABASE,
};
use lmdb::{DatabaseFlags, WriteFlags};
use lmdb_sys::{MDB_cursor_op, MDB_FIRST, MDB_NEXT};
use rsnano_core::{
    utils::{BufferReader, Deserialize},
    Amount, PublicKey,
};
use rsnano_nullable_lmdb::ConfiguredDatabase;
#[cfg(feature = "output_tracking")]
use rsnano_output_tracker::{OutputListenerMt, OutputTrackerMt};
use std::sync::Arc;

pub struct LmdbRepWeightStore {
    _env: Arc<LmdbEnv>,
    database: LmdbDatabase,
    #[cfg(feature = "output_tracking")]
    delete_listener: OutputListenerMt<PublicKey>,
    #[cfg(feature = "output_tracking")]
    put_listener: OutputListenerMt<(PublicKey, Amount)>,
}

impl LmdbRepWeightStore {
    pub fn new(env: Arc<LmdbEnv>) -> anyhow::Result<Self> {
        let database = env
            .environment
            .create_db(Some("rep_weights"), DatabaseFlags::empty())?;
        Ok(Self {
            _env: env,
            database,
            #[cfg(feature = "output_tracking")]
            delete_listener: OutputListenerMt::new(),
            #[cfg(feature = "output_tracking")]
            put_listener: OutputListenerMt::new(),
        })
    }

    #[cfg(feature = "output_tracking")]
    pub fn track_deletions(&self) -> Arc<OutputTrackerMt<PublicKey>> {
        self.delete_listener.track()
    }

    #[cfg(feature = "output_tracking")]
    pub fn track_puts(&self) -> Arc<OutputTrackerMt<(PublicKey, Amount)>> {
        self.put_listener.track()
    }

    pub fn get(&self, txn: &dyn Transaction, pub_key: &PublicKey) -> Option<Amount> {
        match txn.get(self.database, pub_key.as_bytes()) {
            Ok(bytes) => {
                let mut stream = BufferReader::new(bytes);
                Amount::deserialize(&mut stream).ok()
            }
            Err(lmdb::Error::NotFound) => None,
            Err(e) => {
                panic!("Could not load rep_weight: {:?}", e);
            }
        }
    }

    pub fn put(&self, txn: &mut LmdbWriteTransaction, representative: PublicKey, weight: Amount) {
        #[cfg(feature = "output_tracking")]
        self.put_listener.emit((representative, weight));

        txn.put(
            self.database,
            representative.as_bytes(),
            &weight.to_be_bytes(),
            WriteFlags::empty(),
        )
        .unwrap();
    }

    pub fn del(&self, txn: &mut LmdbWriteTransaction, representative: &PublicKey) {
        #[cfg(feature = "output_tracking")]
        self.delete_listener.emit(*representative);

        txn.delete(self.database, representative.as_bytes(), None)
            .unwrap();
    }

    pub fn count(&self, txn: &dyn Transaction) -> u64 {
        txn.count(self.database)
    }

    pub fn iter<'a>(&self, txn: &'a dyn Transaction) -> RepWeightIterator<'a> {
        let cursor = txn.open_ro_cursor(self.database).unwrap();
        RepWeightIterator {
            cursor,
            operation: MDB_FIRST,
        }
    }
}

pub struct RepWeightIterator<'txn> {
    cursor: RoCursor<'txn>,
    operation: MDB_cursor_op,
}

impl<'txn> Iterator for RepWeightIterator<'txn> {
    type Item = (PublicKey, Amount);

    fn next(&mut self) -> Option<Self::Item> {
        match self.cursor.get(None, None, self.operation) {
            Err(lmdb::Error::NotFound) => None,
            Ok((Some(k), v)) => {
                self.operation = MDB_NEXT;
                Some((
                    PublicKey::from_slice(k).unwrap(),
                    Amount::from_be_bytes(v.try_into().unwrap()),
                ))
            }
            Ok(_) => unreachable!(),
            Err(_) => unreachable!(),
        }
    }
}

pub struct ConfiguredRepWeightDatabaseBuilder {
    database: ConfiguredDatabase,
}

impl ConfiguredRepWeightDatabaseBuilder {
    pub fn new() -> Self {
        Self {
            database: ConfiguredDatabase::new(REP_WEIGHT_TEST_DATABASE, "rep_weights"),
        }
    }

    pub fn entry(mut self, account: PublicKey, weight: Amount) -> Self {
        self.database
            .entries
            .insert(account.as_bytes().to_vec(), weight.to_be_bytes().to_vec());
        self
    }

    pub fn build(self) -> ConfiguredDatabase {
        self.database
    }

    pub fn create(hashes: Vec<(PublicKey, Amount)>) -> ConfiguredDatabase {
        let mut builder = Self::new();
        for (account, weight) in hashes {
            builder = builder.entry(account, weight);
        }
        builder.build()
    }
}

#[cfg(test)]
mod tests {
    use lmdb::WriteFlags;

    use super::*;
    use crate::{DeleteEvent, LmdbEnv, PutEvent};

    #[test]
    fn count() {
        let fixture =
            Fixture::with_stored_data(vec![(1.into(), 100.into()), (2.into(), 200.into())]);
        let txn = fixture.env.tx_begin_read();

        assert_eq!(fixture.store.count(&txn), 2);
    }

    #[test]
    fn put() {
        let fixture = Fixture::new();
        let mut txn = fixture.env.tx_begin_write();
        let put_tracker = txn.track_puts();
        let account = PublicKey::from(1);
        let weight = Amount::from(42);

        fixture.store.put(&mut txn, account, weight);

        assert_eq!(
            put_tracker.output(),
            vec![PutEvent {
                database: REP_WEIGHT_TEST_DATABASE.into(),
                key: account.as_bytes().to_vec(),
                value: weight.to_be_bytes().to_vec(),
                flags: WriteFlags::empty()
            }]
        );
    }

    #[test]
    fn load_weight() {
        let account = PublicKey::from(1);
        let weight = Amount::from(42);
        let fixture = Fixture::with_stored_data(vec![(account, weight)]);
        let txn = fixture.env.tx_begin_read();

        let result = fixture.store.get(&txn, &account);

        assert_eq!(result, Some(weight));
    }

    #[test]
    fn delete() {
        let fixture = Fixture::new();
        let mut txn = fixture.env.tx_begin_write();
        let delete_tracker = txn.track_deletions();
        let account = PublicKey::from(1);

        fixture.store.del(&mut txn, &account);

        assert_eq!(
            delete_tracker.output(),
            vec![DeleteEvent {
                database: REP_WEIGHT_TEST_DATABASE.into(),
                key: account.as_bytes().to_vec()
            }]
        )
    }

    #[test]
    fn iter_empty() {
        let fixture = Fixture::new();
        let txn = fixture.env.tx_begin_read();
        let mut iter = fixture.store.iter(&txn);
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn iter() {
        let account1 = PublicKey::from(1);
        let account2 = PublicKey::from(2);
        let weight1 = Amount::from(100);
        let weight2 = Amount::from(200);
        let fixture = Fixture::with_stored_data(vec![(account1, weight1), (account2, weight2)]);

        let txn = fixture.env.tx_begin_read();
        let mut iter = fixture.store.iter(&txn);
        assert_eq!(iter.next(), Some((account1, weight1)));
        assert_eq!(iter.next(), Some((account2, weight2)));
        assert_eq!(iter.next(), None);
    }

    struct Fixture {
        env: Arc<LmdbEnv>,
        store: LmdbRepWeightStore,
    }

    impl Fixture {
        pub fn new() -> Self {
            Self::with_stored_data(Vec::new())
        }

        pub fn with_stored_data(entries: Vec<(PublicKey, Amount)>) -> Self {
            let env = LmdbEnv::new_null_with()
                .configured_database(ConfiguredRepWeightDatabaseBuilder::create(entries))
                .build();
            let env = Arc::new(env);
            Self {
                env: env.clone(),
                store: LmdbRepWeightStore::new(env).unwrap(),
            }
        }
    }
}
