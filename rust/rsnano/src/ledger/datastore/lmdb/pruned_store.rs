use std::sync::Arc;

use lmdb::{Database, DatabaseFlags, WriteFlags};
use rand::{thread_rng, Rng};

use crate::{
    core::BlockHash,
    ledger::datastore::{
        parallel_traversal, pruned_store::PrunedIterator, PrunedStore, ReadTransaction,
        Transaction, WriteTransaction,
    },
};

use super::{as_write_txn, count, exists, LmdbEnv, LmdbIteratorImpl};

pub struct LmdbPrunedStore {
    env: Arc<LmdbEnv>,
    database: Database,
}

impl LmdbPrunedStore {
    pub fn new(env: Arc<LmdbEnv>) -> anyhow::Result<Self> {
        let database = env
            .environment
            .create_db(Some("pruned"), DatabaseFlags::empty())?;
        Ok(Self { env, database })
    }

    pub fn database(&self) -> Database {
        self.database
    }
}

impl PrunedStore for LmdbPrunedStore {
    fn put(&self, txn: &mut dyn WriteTransaction, hash: &BlockHash) {
        as_write_txn(txn)
            .put(self.database, hash.as_bytes(), &[0; 0], WriteFlags::empty())
            .unwrap();
    }

    fn del(&self, txn: &mut dyn WriteTransaction, hash: &BlockHash) {
        as_write_txn(txn)
            .del(self.database, hash.as_bytes(), None)
            .unwrap();
    }

    fn exists(&self, txn: &dyn Transaction, hash: &BlockHash) -> bool {
        exists(txn, self.database, hash.as_bytes())
    }

    fn begin(&self, txn: &dyn Transaction) -> PrunedIterator {
        LmdbIteratorImpl::new_iterator(txn, self.database, None, true)
    }

    fn begin_at_hash(&self, txn: &dyn Transaction, hash: &BlockHash) -> PrunedIterator {
        LmdbIteratorImpl::new_iterator(txn, self.database, Some(hash.as_bytes()), true)
    }

    fn random(&self, txn: &dyn Transaction) -> Option<BlockHash> {
        let random_hash = BlockHash::from_bytes(thread_rng().gen());
        let mut existing = self.begin_at_hash(txn, &random_hash);
        if existing.is_end() {
            existing = self.begin(txn);
        }

        existing.current().map(|(k, _)| *k)
    }

    fn count(&self, txn: &dyn Transaction) -> usize {
        count(txn, self.database)
    }

    fn clear(&self, txn: &mut dyn WriteTransaction) {
        as_write_txn(txn).clear_db(self.database).unwrap();
    }

    fn end(&self) -> PrunedIterator {
        LmdbIteratorImpl::null_iterator()
    }

    fn for_each_par(
        &self,
        action: &(dyn Fn(&dyn ReadTransaction, PrunedIterator, PrunedIterator) + Send + Sync),
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::NoValue, ledger::datastore::lmdb::TestLmdbEnv};

    fn create_sut() -> anyhow::Result<(TestLmdbEnv, LmdbPrunedStore)> {
        let env = TestLmdbEnv::new();
        let store = LmdbPrunedStore::new(env.env())?;
        Ok((env, store))
    }

    #[test]
    fn empty_store() -> anyhow::Result<()> {
        let (env, store) = create_sut()?;
        let txn = env.tx_begin_read()?;

        assert_eq!(store.count(&txn), 0);
        assert_eq!(store.exists(&txn, &BlockHash::from(1)), false);
        assert_eq!(store.begin(&txn).is_end(), true);
        assert!(store.random(&txn).is_none());
        Ok(())
    }

    #[test]
    fn add_one() -> anyhow::Result<()> {
        let (env, store) = create_sut()?;
        let mut txn = env.tx_begin_write()?;

        let hash = BlockHash::from(1);
        store.put(&mut txn, &hash);

        assert_eq!(store.count(&txn), 1);
        assert_eq!(store.exists(&txn, &hash), true);
        assert_eq!(store.begin(&txn).current(), Some((&hash, &NoValue {})));
        assert_eq!(store.random(&txn), Some(hash));
        Ok(())
    }

    #[test]
    fn add_two() -> anyhow::Result<()> {
        let (env, store) = create_sut()?;
        let mut txn = env.tx_begin_write()?;

        let hash1 = BlockHash::from(1);
        let hash2 = BlockHash::from(2);
        store.put(&mut txn, &hash1);
        store.put(&mut txn, &hash2);

        assert_eq!(store.count(&txn), 2);
        assert_eq!(store.exists(&txn, &hash1), true);
        assert_eq!(store.exists(&txn, &hash2), true);
        Ok(())
    }

    #[test]
    fn add_delete() -> anyhow::Result<()> {
        let (env, store) = create_sut()?;
        let mut txn = env.tx_begin_write()?;

        let hash1 = BlockHash::from(1);
        let hash2 = BlockHash::from(2);
        store.put(&mut txn, &hash1);
        store.put(&mut txn, &hash2);
        store.del(&mut txn, &hash1);

        assert_eq!(store.count(&txn), 1);
        assert_eq!(store.exists(&txn, &hash1), false);
        assert_eq!(store.exists(&txn, &hash2), true);
        Ok(())
    }

    #[test]
    fn pruned_random() -> anyhow::Result<()> {
        let (env, store) = create_sut()?;
        let mut txn = env.tx_begin_write()?;
        let hash = BlockHash::random();
        store.put(&mut txn, &hash);
        let random_hash = store.random(txn.txn());
        assert_eq!(random_hash, Some(hash));
        Ok(())
    }
}
