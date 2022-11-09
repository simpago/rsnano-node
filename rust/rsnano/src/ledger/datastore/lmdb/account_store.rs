use crate::{
    core::{Account, AccountInfo},
    ledger::datastore::{
        parallel_traversal, AccountIterator, AccountStore, ReadTransaction, Transaction,
        WriteTransaction,
    },
    utils::{Deserialize, StreamAdapter},
};
use lmdb::{Database, DatabaseFlags, WriteFlags};
use std::sync::Arc;

use super::{as_write_txn, count, get, iterator::LmdbIteratorImpl, LmdbEnv};

pub struct LmdbAccountStore {
    env: Arc<LmdbEnv>,

    /// U256 (arbitrary key) -> blob
    database: Database,
}

impl LmdbAccountStore {
    pub fn new(env: Arc<LmdbEnv>) -> anyhow::Result<Self> {
        let database = env
            .environment
            .create_db(Some("accounts"), DatabaseFlags::empty())?;
        Ok(Self { env, database })
    }

    pub fn database(&self) -> Database {
        self.database
    }
}

impl AccountStore for LmdbAccountStore {
    fn put(&self, transaction: &mut dyn WriteTransaction, account: &Account, info: &AccountInfo) {
        as_write_txn(transaction)
            .put(
                self.database,
                account.as_bytes(),
                &info.to_bytes(),
                WriteFlags::empty(),
            )
            .unwrap();
    }

    fn get(&self, transaction: &dyn Transaction, account: &Account) -> Option<AccountInfo> {
        let result = get(transaction, self.database, account.as_bytes());
        match result {
            Err(lmdb::Error::NotFound) => None,
            Ok(bytes) => {
                let mut stream = StreamAdapter::new(bytes);
                AccountInfo::deserialize(&mut stream).ok()
            }
            Err(e) => panic!("Could not load account info {:?}", e),
        }
    }

    fn del(&self, transaction: &mut dyn WriteTransaction, account: &Account) {
        as_write_txn(transaction)
            .del(self.database, account.as_bytes(), None)
            .unwrap();
    }

    fn begin_account(&self, transaction: &dyn Transaction, account: &Account) -> AccountIterator {
        LmdbIteratorImpl::new_iterator(transaction, self.database, Some(account.as_bytes()), true)
    }

    fn begin(&self, transaction: &dyn Transaction) -> AccountIterator {
        LmdbIteratorImpl::new_iterator(transaction, self.database, None, true)
    }

    fn for_each_par(
        &self,
        action: &(dyn Fn(&dyn ReadTransaction, AccountIterator, AccountIterator) + Send + Sync),
    ) {
        parallel_traversal(&|start, end, is_last| {
            let txn = self.env.tx_begin_read().unwrap();
            let begin_it = self.begin_account(&txn, &start.into());
            let end_it = if !is_last {
                self.begin_account(&txn, &end.into())
            } else {
                self.end()
            };
            action(&txn, begin_it, end_it);
        })
    }

    fn end(&self) -> AccountIterator {
        LmdbIteratorImpl::null_iterator()
    }

    fn count(&self, txn: &dyn Transaction) -> usize {
        count(txn, self.database)
    }

    fn exists(&self, txn: &dyn Transaction, account: &Account) -> bool {
        !self.begin_account(txn, account).is_end()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::{
        core::{Amount, BlockHash},
        ledger::datastore::lmdb::TestLmdbEnv,
    };

    struct AccountStoreTestContext {
        pub store: LmdbAccountStore,
        pub env: TestLmdbEnv,
    }

    impl AccountStoreTestContext {
        pub fn new() -> Self {
            let env = TestLmdbEnv::new();
            let store = LmdbAccountStore::new(env.env()).unwrap();
            Self { store, env }
        }
    }

    #[test]
    fn empty_store() {
        let sut = AccountStoreTestContext::new();
        let txn = sut.env.tx_begin_read().unwrap();
        let account = Account::from(1);
        let result = sut.store.get(&txn, &account);
        assert_eq!(result, None);
        assert_eq!(sut.store.exists(&txn, &account), false);
        assert_eq!(sut.store.count(&txn), 0);
    }

    #[test]
    fn add_one_account() {
        let sut = AccountStoreTestContext::new();
        let mut txn = sut.env.tx_begin_write().unwrap();
        let account = Account::from(1);
        let info = AccountInfo {
            balance: Amount::new(123),
            ..Default::default()
        };
        sut.store.put(&mut txn, &account, &info);
        assert!(sut.store.exists(&txn, &account));
        let result = sut.store.get(&txn, &account);
        assert_eq!(result, Some(info));
        assert_eq!(sut.store.count(&txn), 1);
    }

    #[test]
    fn del() {
        let sut = AccountStoreTestContext::new();
        let mut txn = sut.env.tx_begin_write().unwrap();
        let account = Account::from(1);
        let info = AccountInfo {
            balance: Amount::new(123),
            ..Default::default()
        };
        sut.store.put(&mut txn, &account, &info);
        sut.store.del(&mut txn, &account);
        let result = sut.store.get(&txn, &account);
        assert_eq!(result, None);
    }

    #[test]
    fn begin_empty_store() {
        let sut = AccountStoreTestContext::new();
        let txn = sut.env.tx_begin_read().unwrap();
        let it = sut.store.begin(&txn);
        assert!(it.is_end())
    }

    #[test]
    fn begin() {
        let sut = AccountStoreTestContext::new();
        let mut txn = sut.env.tx_begin_write().unwrap();
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
        sut.store.put(&mut txn, &account1, &info1);
        sut.store.put(&mut txn, &account2, &info2);
        let mut it = sut.store.begin(&txn);
        assert_eq!(it.current(), Some((&account1, &info1)));
        it.next();
        assert_eq!(it.current(), Some((&account2, &info2)));
        it.next();
        assert_eq!(it.current(), None);
    }

    #[test]
    fn begin_account() {
        let sut = AccountStoreTestContext::new();
        let mut txn = sut.env.tx_begin_write().unwrap();
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
        sut.store.put(&mut txn, &account1, &info1);
        sut.store.put(&mut txn, &account3, &info3);
        let mut it = sut.store.begin_account(&txn, &Account::from(2));
        assert_eq!(it.current(), Some((&account3, &info3)));
        it.next();
        assert_eq!(it.current(), None);
    }

    #[test]
    fn for_each_par() {
        let sut = AccountStoreTestContext::new();
        let mut txn = sut.env.tx_begin_write().unwrap();
        let account_1 = Account::from(1);
        let account_max = Account::from_bytes([0xFF; 32]);
        let info_1 = AccountInfo {
            balance: Amount::new(1),
            ..Default::default()
        };
        let info_max = AccountInfo {
            balance: Amount::new(3),
            ..Default::default()
        };
        sut.store.put(&mut txn, &account_1, &info_1);
        sut.store.put(&mut txn, &account_max, &info_max);
        txn.commit();

        let balance_sum = Mutex::new(Amount::zero());
        sut.store.for_each_par(&|_, mut begin, end| {
            while !begin.eq(end.as_ref()) {
                if let Some((_, v)) = begin.current() {
                    *balance_sum.lock().unwrap() += v.balance
                }
                begin.next();
            }
        });
        assert_eq!(*balance_sum.lock().unwrap(), Amount::new(4));
    }
}
