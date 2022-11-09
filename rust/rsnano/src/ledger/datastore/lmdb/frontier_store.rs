use lmdb::{Database, DatabaseFlags, WriteFlags};
use std::sync::Arc;

use crate::{
    core::{Account, BlockHash},
    ledger::datastore::{
        frontier_store::FrontierIterator, parallel_traversal, FrontierStore, ReadTransaction,
        Transaction, WriteTransaction,
    },
};

use super::{as_write_txn, get, LmdbEnv, LmdbIteratorImpl};

pub struct LmdbFrontierStore {
    env: Arc<LmdbEnv>,
    database: Database,
}

impl LmdbFrontierStore {
    pub fn new(env: Arc<LmdbEnv>) -> anyhow::Result<Self> {
        let database = env
            .environment
            .create_db(Some("frontiers"), DatabaseFlags::empty())?;
        Ok(Self { env, database })
    }

    pub fn database(&self) -> Database {
        self.database
    }

    pub fn create_db(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

impl FrontierStore for LmdbFrontierStore {
    fn put(&self, txn: &mut dyn WriteTransaction, hash: &BlockHash, account: &Account) {
        as_write_txn(txn)
            .put(
                self.database,
                hash.as_bytes(),
                account.as_bytes(),
                WriteFlags::empty(),
            )
            .unwrap();
    }

    fn get(&self, txn: &dyn Transaction, hash: &BlockHash) -> Account {
        match get(txn, self.database, hash.as_bytes()) {
            Ok(bytes) => Account::from_slice(bytes).unwrap_or_default(),
            Err(lmdb::Error::NotFound) => Account::zero(),
            Err(e) => panic!("Could not load frontier: {:?}", e),
        }
    }

    fn del(&self, txn: &mut dyn WriteTransaction, hash: &BlockHash) {
        as_write_txn(txn)
            .del(self.database, hash.as_bytes(), None)
            .unwrap();
    }

    fn begin(&self, txn: &dyn Transaction) -> FrontierIterator {
        LmdbIteratorImpl::new_iterator(txn, self.database, None, true)
    }

    fn begin_at_hash(&self, txn: &dyn Transaction, hash: &BlockHash) -> FrontierIterator {
        LmdbIteratorImpl::new_iterator(txn, self.database, Some(hash.as_bytes()), true)
    }

    fn for_each_par(
        &self,
        action: &(dyn Fn(&dyn ReadTransaction, FrontierIterator, FrontierIterator) + Send + Sync),
    ) {
        parallel_traversal(&|start, end, is_last| {
            let transaction = self.env.tx_begin_read().unwrap();
            let begin_it = self.begin_at_hash(&transaction, &start.into());
            let end_it = if !is_last {
                self.begin_at_hash(&transaction, &end.into())
            } else {
                self.end()
            };
            action(&transaction, begin_it, end_it);
        });
    }

    fn end(&self) -> FrontierIterator {
        LmdbIteratorImpl::null_iterator()
    }
}

#[cfg(test)]
mod tests {
    use crate::ledger::datastore::lmdb::TestLmdbEnv;

    use super::*;

    #[test]
    fn empty_store() -> anyhow::Result<()> {
        let env = TestLmdbEnv::new();
        let store = LmdbFrontierStore::new(env.env())?;
        let txn = env.tx_begin_read()?;
        assert_eq!(store.get(&txn, &BlockHash::from(1)), Account::zero());
        assert!(store.begin(&txn).is_end());
        Ok(())
    }

    #[test]
    fn put() -> anyhow::Result<()> {
        let env = TestLmdbEnv::new();
        let store = LmdbFrontierStore::new(env.env())?;
        let mut txn = env.tx_begin_write()?;
        let block = BlockHash::from(1);
        let account = Account::from(2);

        store.put(&mut txn, &block, &account);
        let loaded = store.get(&txn, &block);

        assert_eq!(loaded, account);
        Ok(())
    }

    #[test]
    fn delete() -> anyhow::Result<()> {
        let env = TestLmdbEnv::new();
        let store = LmdbFrontierStore::new(env.env())?;
        let mut txn = env.tx_begin_write()?;
        let block = BlockHash::from(1);
        store.put(&mut txn, &block, &Account::from(2));

        store.del(&mut txn, &block);

        let loaded = store.get(&txn, &block);
        assert_eq!(loaded, Account::zero());
        Ok(())
    }
}
