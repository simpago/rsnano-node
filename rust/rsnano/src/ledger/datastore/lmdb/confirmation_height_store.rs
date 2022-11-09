use lmdb::{Database, DatabaseFlags, WriteFlags};
use std::sync::Arc;

use crate::{
    core::{Account, ConfirmationHeightInfo},
    ledger::datastore::{
        confirmation_height_store::ConfirmationHeightIterator, parallel_traversal,
        ConfirmationHeightStore, ReadTransaction, Transaction, WriteTransaction,
    },
    utils::{Deserialize, StreamAdapter},
};

use super::{as_write_txn, count, exists, get, LmdbEnv, LmdbIteratorImpl};

pub struct LmdbConfirmationHeightStore {
    env: Arc<LmdbEnv>,
    database: Database,
}

impl LmdbConfirmationHeightStore {
    pub fn new(env: Arc<LmdbEnv>) -> anyhow::Result<Self> {
        let database = env
            .environment
            .create_db(Some("confirmation_height"), DatabaseFlags::empty())?;
        Ok(Self { env, database })
    }

    pub fn database(&self) -> Database {
        self.database
    }
}

impl ConfirmationHeightStore for LmdbConfirmationHeightStore {
    fn put(
        &self,
        txn: &mut dyn WriteTransaction,
        account: &Account,
        info: &ConfirmationHeightInfo,
    ) {
        as_write_txn(txn)
            .put(
                self.database,
                account.as_bytes(),
                &info.to_bytes(),
                WriteFlags::empty(),
            )
            .unwrap();
    }

    fn get(&self, txn: &dyn Transaction, account: &Account) -> Option<ConfirmationHeightInfo> {
        match get(txn, self.database, account.as_bytes()) {
            Err(lmdb::Error::NotFound) => None,
            Ok(bytes) => {
                let mut stream = StreamAdapter::new(bytes);
                ConfirmationHeightInfo::deserialize(&mut stream).ok()
            }
            Err(e) => {
                panic!("Could not load confirmation height info: {:?}", e);
            }
        }
    }

    fn exists(&self, txn: &dyn Transaction, account: &Account) -> bool {
        exists(txn, self.database, account.as_bytes())
    }

    fn del(&self, txn: &mut dyn WriteTransaction, account: &Account) {
        as_write_txn(txn)
            .del(self.database, account.as_bytes(), None)
            .unwrap();
    }

    fn count(&self, txn: &dyn Transaction) -> usize {
        count(txn, self.database)
    }

    fn clear(&self, txn: &mut dyn WriteTransaction) {
        as_write_txn(txn).clear_db(self.database).unwrap()
    }

    fn begin(&self, txn: &dyn Transaction) -> ConfirmationHeightIterator {
        LmdbIteratorImpl::new_iterator(txn, self.database, None, true)
    }

    fn begin_at_account(
        &self,
        txn: &dyn Transaction,
        account: &Account,
    ) -> ConfirmationHeightIterator {
        LmdbIteratorImpl::new_iterator(txn, self.database, Some(account.as_bytes()), true)
    }

    fn end(&self) -> ConfirmationHeightIterator {
        LmdbIteratorImpl::null_iterator()
    }

    fn for_each_par(
        &self,
        action: &(dyn Fn(&dyn ReadTransaction, ConfirmationHeightIterator, ConfirmationHeightIterator)
              + Send
              + Sync),
    ) {
        parallel_traversal(&|start, end, is_last| {
            let transaction = self.env.tx_begin_read().unwrap();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::BlockHash, ledger::datastore::lmdb::TestLmdbEnv};

    #[test]
    fn empty_store() -> anyhow::Result<()> {
        let env = TestLmdbEnv::new();
        let store = LmdbConfirmationHeightStore::new(env.env())?;
        let txn = env.tx_begin_read()?;
        assert!(store.get(&txn, &Account::from(0)).is_none());
        assert_eq!(store.exists(&txn, &Account::from(0)), false);
        assert!(store.begin(&txn).is_end());
        assert!(store.begin_at_account(&txn, &Account::from(0)).is_end());
        Ok(())
    }

    #[test]
    fn add_account() -> anyhow::Result<()> {
        let env = TestLmdbEnv::new();
        let store = LmdbConfirmationHeightStore::new(env.env())?;
        let mut txn = env.tx_begin_write()?;
        let account = Account::from(1);
        let info = ConfirmationHeightInfo::new(1, BlockHash::from(2));
        store.put(&mut txn, &account, &info);
        let loaded = store.get(&txn, &account);
        assert_eq!(loaded, Some(info));
        Ok(())
    }

    #[test]
    fn iterate_one_account() -> anyhow::Result<()> {
        let env = TestLmdbEnv::new();
        let store = LmdbConfirmationHeightStore::new(env.env())?;
        let mut txn = env.tx_begin_write()?;
        let account = Account::from(1);
        let info = ConfirmationHeightInfo::new(1, BlockHash::from(2));
        store.put(&mut txn, &account, &info);

        let mut it = store.begin(&txn);
        assert_eq!(it.current(), Some((&account, &info)));

        it.next();
        assert!(it.is_end());
        Ok(())
    }

    #[test]
    fn iterate_two_accounts() -> anyhow::Result<()> {
        let env = TestLmdbEnv::new();
        let store = LmdbConfirmationHeightStore::new(env.env())?;
        let mut txn = env.tx_begin_write()?;
        let account1 = Account::from(1);
        let account2 = Account::from(2);
        let info1 = ConfirmationHeightInfo::new(1, BlockHash::from(2));
        let info2 = ConfirmationHeightInfo::new(3, BlockHash::from(4));
        store.put(&mut txn, &account1, &info1);
        store.put(&mut txn, &account2, &info2);

        let mut it = store.begin(&txn);
        assert_eq!(it.current(), Some((&account1, &info1)));
        it.next();
        assert_eq!(it.current(), Some((&account2, &info2)));
        it.next();
        assert!(it.is_end());
        Ok(())
    }

    #[test]
    fn clear() -> anyhow::Result<()> {
        let env = TestLmdbEnv::new();
        let store = LmdbConfirmationHeightStore::new(env.env())?;
        let mut txn = env.tx_begin_write()?;
        let account1 = Account::from(1);
        let account2 = Account::from(2);
        let info1 = ConfirmationHeightInfo::new(1, BlockHash::from(2));
        let info2 = ConfirmationHeightInfo::new(3, BlockHash::from(4));
        store.put(&mut txn, &account1, &info1);
        store.put(&mut txn, &account2, &info2);

        store.clear(&mut txn);

        assert_eq!(store.count(&txn), 0);
        Ok(())
    }
}
