use crate::{
    iterator::{LmdbIterator, LmdbRangeIterator},
    parallel_traversal, LmdbDatabase, LmdbEnv, LmdbWriteTransaction, Transaction,
    ACCOUNT_TEST_DATABASE,
};
use lmdb::{DatabaseFlags, WriteFlags};
use rsnano_core::{
    utils::{BufferReader, Deserialize},
    Account, AccountInfo,
};
use rsnano_nullable_lmdb::ConfiguredDatabase;
#[cfg(feature = "output_tracking")]
use rsnano_output_tracker::{OutputListenerMt, OutputTrackerMt};
use std::{ops::RangeBounds, sync::Arc};

pub struct LmdbAccountStore {
    env: Arc<LmdbEnv>,

    /// U256 (arbitrary key) -> blob
    database: LmdbDatabase,
    #[cfg(feature = "output_tracking")]
    put_listener: OutputListenerMt<(Account, AccountInfo)>,
}

impl LmdbAccountStore {
    pub fn new(env: Arc<LmdbEnv>) -> anyhow::Result<Self> {
        let database = env
            .environment
            .create_db(Some("accounts"), DatabaseFlags::empty())?;
        Ok(Self {
            env,
            database,
            #[cfg(feature = "output_tracking")]
            put_listener: OutputListenerMt::new(),
        })
    }

    #[cfg(feature = "output_tracking")]
    pub fn track_puts(&self) -> Arc<OutputTrackerMt<(Account, AccountInfo)>> {
        self.put_listener.track()
    }

    pub fn database(&self) -> LmdbDatabase {
        self.database
    }

    pub fn put(
        &self,
        transaction: &mut LmdbWriteTransaction,
        account: &Account,
        info: &AccountInfo,
    ) {
        #[cfg(feature = "output_tracking")]
        self.put_listener.emit((*account, info.clone()));
        transaction
            .put(
                self.database,
                account.as_bytes(),
                &info.to_bytes(),
                WriteFlags::empty(),
            )
            .unwrap();
    }

    pub fn get(&self, transaction: &dyn Transaction, account: &Account) -> Option<AccountInfo> {
        let result = transaction.get(self.database, account.as_bytes());
        match result {
            Err(lmdb::Error::NotFound) => None,
            Ok(bytes) => {
                let mut stream = BufferReader::new(bytes);
                AccountInfo::deserialize(&mut stream).ok()
            }
            Err(e) => panic!("Could not load account info {:?}", e),
        }
    }

    pub fn del(&self, transaction: &mut LmdbWriteTransaction, account: &Account) {
        transaction
            .delete(self.database, account.as_bytes(), None)
            .unwrap();
    }

    pub fn iter<'txn>(
        &self,
        tx: &'txn dyn Transaction,
    ) -> impl Iterator<Item = (Account, AccountInfo)> + 'txn {
        let cursor = tx
            .open_ro_cursor(self.database)
            .expect("could not read from account store");

        LmdbIterator::new(cursor, |key, value| {
            let account = Account::from_bytes(key.try_into().unwrap());
            let mut stream = BufferReader::new(value);
            let info = AccountInfo::deserialize(&mut stream).unwrap();
            (account, info)
        })
    }

    pub fn iter_range<'txn>(
        &self,
        tx: &'txn dyn Transaction,
        range: impl RangeBounds<Account> + 'static,
    ) -> impl Iterator<Item = (Account, AccountInfo)> + 'txn {
        let cursor = tx.open_ro_cursor(self.database).unwrap();
        LmdbRangeIterator::new(cursor, range)
    }

    pub fn for_each_par(
        &self,
        action: impl Fn(&mut dyn Iterator<Item = (Account, AccountInfo)>) + Send + Sync,
    ) {
        parallel_traversal(&|start, end, is_last| {
            let tx = self.env.tx_begin_read();
            let start_account = Account::from(start);
            let end_account = Account::from(end);
            if is_last {
                let mut iter = self.iter_range(&tx, start_account..);
                action(&mut iter);
            } else {
                let mut iter = self.iter_range(&tx, start_account..end_account);
                action(&mut iter);
            }
        })
    }

    pub fn count(&self, txn: &dyn Transaction) -> u64 {
        txn.count(self.database)
    }
}

pub struct ConfiguredAccountDatabaseBuilder {
    database: ConfiguredDatabase,
}

impl ConfiguredAccountDatabaseBuilder {
    pub fn new() -> Self {
        Self {
            database: ConfiguredDatabase::new(ACCOUNT_TEST_DATABASE, "accounts"),
        }
    }

    pub fn account(mut self, account: &Account, info: &AccountInfo) -> Self {
        self.database
            .entries
            .insert(account.as_bytes().to_vec(), info.to_bytes().to_vec());
        self
    }

    pub fn build(self) -> ConfiguredDatabase {
        self.database
    }

    pub fn create(frontiers: Vec<(Account, AccountInfo)>) -> ConfiguredDatabase {
        let mut builder = Self::new();
        for (account, info) in frontiers {
            builder = builder.account(&account, &info);
        }
        builder.build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DeleteEvent, PutEvent};
    use rsnano_core::{Amount, BlockHash};
    use std::sync::Mutex;

    struct Fixture {
        env: Arc<LmdbEnv>,
        store: LmdbAccountStore,
    }

    impl Fixture {
        fn new() -> Self {
            Self::with_stored_accounts(Vec::new())
        }

        fn with_stored_accounts(accounts: Vec<(Account, AccountInfo)>) -> Self {
            let env = LmdbEnv::new_null_with()
                .configured_database(ConfiguredAccountDatabaseBuilder::create(accounts))
                .build();
            Self::with_env(env)
        }

        fn with_env(env: LmdbEnv) -> Self {
            let env = Arc::new(env);
            let store = LmdbAccountStore::new(env.clone()).unwrap();

            Fixture { env, store }
        }
    }

    #[test]
    fn empty_store() {
        let fixture = Fixture::new();
        let txn = fixture.env.tx_begin_read();
        let account = Account::from(1);
        let result = fixture.store.get(&txn, &account);
        assert_eq!(result, None);
        assert_eq!(fixture.store.count(&txn), 0);
    }

    #[test]
    fn add_one_account() {
        let fixture = Fixture::new();
        let mut txn = fixture.env.tx_begin_write();
        let put_tracker = txn.track_puts();

        let account = Account::from(1);
        let info = AccountInfo::new_test_instance();
        fixture.store.put(&mut txn, &account, &info);

        assert_eq!(
            put_tracker.output(),
            vec![PutEvent {
                database: ACCOUNT_TEST_DATABASE.into(),
                key: account.as_bytes().to_vec(),
                value: info.to_bytes().to_vec(),
                flags: lmdb::WriteFlags::empty()
            }]
        );
    }

    #[test]
    fn load_account() {
        let account = Account::from(1);
        let info = AccountInfo::new_test_instance();
        let fixture = Fixture::with_stored_accounts(vec![(account.clone(), info.clone())]);
        let txn = fixture.env.tx_begin_read();

        let result = fixture.store.get(&txn, &account);

        assert_eq!(result, Some(info));
    }

    #[test]
    fn count() {
        let fixture = Fixture::with_stored_accounts(vec![
            (Account::from(1), AccountInfo::new_test_instance()),
            (Account::from(2), AccountInfo::new_test_instance()),
        ]);
        let txn = fixture.env.tx_begin_read();

        let count = fixture.store.count(&txn);

        assert_eq!(count, 2);
    }

    #[test]
    fn delete_account() {
        let fixture = Fixture::new();
        let mut txn = fixture.env.tx_begin_write();
        let delete_tracker = txn.track_deletions();

        let account = Account::from(1);
        fixture.store.del(&mut txn, &account);

        assert_eq!(
            delete_tracker.output(),
            vec![DeleteEvent {
                database: ACCOUNT_TEST_DATABASE.into(),
                key: account.as_bytes().to_vec()
            }]
        )
    }

    #[test]
    fn begin_empty_store_nullable() {
        let fixture = Fixture::new();
        let txn = fixture.env.tx_begin_read();
        let mut it = fixture.store.iter(&txn);
        assert_eq!(it.next(), None);
    }

    #[test]
    fn begin() {
        let account1 = Account::from(1);
        let account2 = Account::from(2);
        let info1 = AccountInfo {
            head: BlockHash::from(1),
            ..Default::default()
        };
        let info2 = AccountInfo {
            head: BlockHash::from(2),
            ..Default::default()
        };

        let fixture = Fixture::with_stored_accounts(vec![
            (account1.clone(), info1.clone()),
            (account2.clone(), info2.clone()),
        ]);
        let txn = fixture.env.tx_begin_read();

        let mut it = fixture.store.iter(&txn);
        assert_eq!(it.next(), Some((account1, info1)));
        assert_eq!(it.next(), Some((account2, info2)));
        assert_eq!(it.next(), None);
    }

    #[test]
    fn begin_account() {
        let account1 = Account::from(1);
        let account3 = Account::from(3);
        let info1 = AccountInfo {
            head: BlockHash::from(1),
            ..Default::default()
        };
        let info3 = AccountInfo {
            head: BlockHash::from(3),
            ..Default::default()
        };

        let fixture = Fixture::with_stored_accounts(vec![
            (account1.clone(), info1.clone()),
            (account3.clone(), info3.clone()),
        ]);
        let txn = fixture.env.tx_begin_read();

        let mut it = fixture.store.iter_range(&txn, Account::from(2)..);

        assert_eq!(it.next(), Some((account3, info3)));
        assert_eq!(it.next(), None);
    }

    #[test]
    fn for_each_par() {
        let account1 = Account::from(1);
        let account3 = Account::from(3);
        let info1 = AccountInfo {
            balance: Amount::raw(1),
            ..Default::default()
        };
        let info3 = AccountInfo {
            balance: Amount::raw(3),
            ..Default::default()
        };

        let fixture = Fixture::with_stored_accounts(vec![
            (account1.clone(), info1.clone()),
            (account3.clone(), info3.clone()),
        ]);

        let balance_sum = Mutex::new(Amount::zero());
        fixture.store.for_each_par(|iter| {
            for (_, info) in iter {
                *balance_sum.lock().unwrap() += info.balance;
            }
        });
        assert_eq!(*balance_sum.lock().unwrap(), Amount::raw(4));
    }

    #[test]
    fn track_inserted_account_info() {
        let fixture = Fixture::new();
        let put_tracker = fixture.store.track_puts();
        let mut txn = fixture.env.tx_begin_write();
        let account = Account::from(1);
        let info = AccountInfo::new_test_instance();

        fixture.store.put(&mut txn, &account, &info);

        assert_eq!(put_tracker.output(), vec![(account, info)]);
    }
}
