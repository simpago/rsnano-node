use crate::{
    parallel_traversal, BinaryDbIterator, LmdbDatabase, LmdbEnv, LmdbIteratorImpl,
    LmdbReadTransaction, LmdbWriteTransaction, Transaction, CONFIRMATION_HEIGHT_TEST_DATABASE,
};
use lmdb::{DatabaseFlags, WriteFlags};
use rsnano_core::{
    utils::{BufferReader, Deserialize},
    Account, ConfirmationHeightInfo,
};
use rsnano_nullable_lmdb::ConfiguredDatabase;
use std::sync::Arc;

pub type ConfirmationHeightIterator<'txn> = BinaryDbIterator<'txn, Account, ConfirmationHeightInfo>;

pub struct LmdbConfirmationHeightStore {
    env: Arc<LmdbEnv>,
    database: LmdbDatabase,
}

impl LmdbConfirmationHeightStore {
    pub fn new(env: Arc<LmdbEnv>) -> anyhow::Result<Self> {
        let database = env
            .environment
            .create_db(Some("confirmation_height"), DatabaseFlags::empty())?;
        Ok(Self { env, database })
    }

    pub fn database(&self) -> LmdbDatabase {
        self.database
    }

    pub fn put(
        &self,
        txn: &mut LmdbWriteTransaction,
        account: &Account,
        info: &ConfirmationHeightInfo,
    ) {
        txn.put(
            self.database,
            account.as_bytes(),
            &info.to_bytes(),
            WriteFlags::empty(),
        )
        .unwrap();
    }

    pub fn get(&self, txn: &dyn Transaction, account: &Account) -> Option<ConfirmationHeightInfo> {
        match txn.get(self.database, account.as_bytes()) {
            Err(lmdb::Error::NotFound) => None,
            Ok(bytes) => {
                let mut stream = BufferReader::new(bytes);
                ConfirmationHeightInfo::deserialize(&mut stream).ok()
            }
            Err(e) => {
                panic!("Could not load confirmation height info: {:?}", e);
            }
        }
    }

    pub fn exists(&self, txn: &dyn Transaction, account: &Account) -> bool {
        txn.exists(self.database, account.as_bytes())
    }

    pub fn del(&self, txn: &mut LmdbWriteTransaction, account: &Account) {
        txn.delete(self.database, account.as_bytes(), None).unwrap();
    }

    pub fn count(&self, txn: &dyn Transaction) -> u64 {
        txn.count(self.database)
    }

    pub fn clear(&self, txn: &mut LmdbWriteTransaction) {
        txn.clear_db(self.database).unwrap()
    }

    pub fn begin<'txn>(&self, txn: &'txn dyn Transaction) -> ConfirmationHeightIterator<'txn> {
        LmdbIteratorImpl::new_iterator(txn, self.database, None, true)
    }

    pub fn begin_at_account<'txn>(
        &self,
        txn: &'txn dyn Transaction,
        account: &Account,
    ) -> ConfirmationHeightIterator<'txn> {
        LmdbIteratorImpl::new_iterator(txn, self.database, Some(account.as_bytes()), true)
    }

    pub fn end(&self) -> ConfirmationHeightIterator {
        LmdbIteratorImpl::null_iterator()
    }

    pub fn for_each_par(
        &self,
        action: &(dyn Fn(&LmdbReadTransaction, ConfirmationHeightIterator, ConfirmationHeightIterator)
              + Send
              + Sync),
    ) {
        parallel_traversal(&|start, end, is_last| {
            let transaction = self.env.tx_begin_read();
            let begin_it = self.begin_at_account(&transaction, &start.into());
            let end_it = if !is_last {
                self.begin_at_account(&transaction, &end.into())
            } else {
                self.end()
            };
            action(&transaction, begin_it, end_it);
        });
    }
}

pub struct ConfiguredConfirmationHeightDatabaseBuilder {
    database: ConfiguredDatabase,
}

impl ConfiguredConfirmationHeightDatabaseBuilder {
    pub fn new() -> Self {
        Self {
            database: ConfiguredDatabase::new(
                CONFIRMATION_HEIGHT_TEST_DATABASE,
                "confirmation_height",
            ),
        }
    }

    pub fn height(mut self, account: &Account, info: &ConfirmationHeightInfo) -> Self {
        self.database
            .entries
            .insert(account.as_bytes().to_vec(), info.to_bytes().to_vec());
        self
    }

    pub fn build(self) -> ConfiguredDatabase {
        self.database
    }

    pub fn create(hashes: Vec<(Account, ConfirmationHeightInfo)>) -> ConfiguredDatabase {
        let mut builder = Self::new();
        for (account, info) in hashes {
            builder = builder.height(&account, &info);
        }
        builder.build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PutEvent;
    use rsnano_core::BlockHash;

    struct Fixture {
        env: Arc<LmdbEnv>,
        store: LmdbConfirmationHeightStore,
    }

    impl Fixture {
        fn new() -> Self {
            Self::with_env(LmdbEnv::new_null())
        }

        fn with_env(env: LmdbEnv) -> Self {
            let env = Arc::new(env);
            Self {
                env: env.clone(),
                store: LmdbConfirmationHeightStore::new(env).unwrap(),
            }
        }
    }

    #[test]
    fn empty_store() {
        let fixture = Fixture::new();
        let store = &fixture.store;
        let txn = fixture.env.tx_begin_read();
        assert!(store.get(&txn, &Account::from(0)).is_none());
        assert_eq!(store.exists(&txn, &Account::from(0)), false);
        assert!(store.begin(&txn).is_end());
        assert!(store.begin_at_account(&txn, &Account::from(0)).is_end());
    }

    #[test]
    fn add_account() {
        let fixture = Fixture::new();
        let mut txn = fixture.env.tx_begin_write();
        let put_tracker = txn.track_puts();

        let account = Account::from(1);
        let info = ConfirmationHeightInfo::new(1, BlockHash::from(2));
        fixture.store.put(&mut txn, &account, &info);

        assert_eq!(
            put_tracker.output(),
            vec![PutEvent {
                database: LmdbDatabase::new_null(42),
                key: account.as_bytes().to_vec(),
                value: info.to_bytes().to_vec(),
                flags: WriteFlags::empty(),
            }]
        )
    }

    #[test]
    fn load() {
        let account = Account::from(1);
        let info = ConfirmationHeightInfo::new(1, BlockHash::from(2));

        let env = LmdbEnv::new_null_with()
            .database("confirmation_height", LmdbDatabase::new_null(100))
            .entry(account.as_bytes(), &info.to_bytes())
            .build()
            .build();

        let fixture = Fixture::with_env(env);
        let txn = fixture.env.tx_begin_read();
        let result = fixture.store.get(&txn, &account);

        assert_eq!(result, Some(info))
    }

    #[test]
    fn iterate_one_account() -> anyhow::Result<()> {
        let account = Account::from(1);
        let info = ConfirmationHeightInfo::new(1, BlockHash::from(2));

        let env = LmdbEnv::new_null_with()
            .database("confirmation_height", LmdbDatabase::new_null(100))
            .entry(account.as_bytes(), &info.to_bytes())
            .build()
            .build();

        let fixture = Fixture::with_env(env);
        let txn = fixture.env.tx_begin_read();
        let mut it = fixture.store.begin(&txn);
        assert_eq!(it.current(), Some((&account, &info)));

        it.next();
        assert!(it.is_end());
        Ok(())
    }

    #[test]
    fn clear() {
        let fixture = Fixture::new();
        let mut txn = fixture.env.tx_begin_write();
        let clear_tracker = txn.track_clears();

        fixture.store.clear(&mut txn);

        assert_eq!(clear_tracker.output(), vec![LmdbDatabase::new_null(42)])
    }
}
