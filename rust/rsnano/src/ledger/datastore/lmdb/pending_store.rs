use std::sync::Arc;

use lmdb::{Database, DatabaseFlags, WriteFlags};

use crate::{
    core::{Account, BlockHash, PendingInfo, PendingKey},
    ledger::datastore::{
        parallel_traversal_u512, pending_store::PendingIterator, PendingStore, ReadTransaction,
        Transaction, WriteTransaction,
    },
    utils::{Deserialize, StreamAdapter},
};

use super::{as_write_txn, get, LmdbEnv, LmdbIteratorImpl};

pub struct LmdbPendingStore {
    env: Arc<LmdbEnv>,
    database: Database,
}

impl LmdbPendingStore {
    pub fn new(env: Arc<LmdbEnv>) -> anyhow::Result<Self> {
        let database = env
            .environment
            .create_db(Some("pending"), DatabaseFlags::empty())?;

        Ok(Self { env, database })
    }

    pub fn database(&self) -> Database {
        self.database
    }
}

impl PendingStore for LmdbPendingStore {
    fn put(&self, txn: &mut dyn WriteTransaction, key: &PendingKey, pending: &PendingInfo) {
        let key_bytes = key.to_bytes();
        let pending_bytes = pending.to_bytes();
        as_write_txn(txn)
            .put(
                self.database,
                &key_bytes,
                &pending_bytes,
                WriteFlags::empty(),
            )
            .unwrap();
    }

    fn del(&self, txn: &mut dyn WriteTransaction, key: &PendingKey) {
        let key_bytes = key.to_bytes();
        as_write_txn(txn)
            .del(self.database, &key_bytes, None)
            .unwrap();
    }

    fn get(&self, txn: &dyn Transaction, key: &PendingKey) -> Option<PendingInfo> {
        let key_bytes = key.to_bytes();
        match get(txn, self.database, &key_bytes) {
            Ok(bytes) => {
                let mut stream = StreamAdapter::new(bytes);
                PendingInfo::deserialize(&mut stream).ok()
            }
            Err(lmdb::Error::NotFound) => None,
            Err(e) => {
                panic!("Could not load pending info: {:?}", e);
            }
        }
    }

    fn begin(&self, txn: &dyn Transaction) -> PendingIterator {
        LmdbIteratorImpl::new_iterator(txn, self.database, None, true)
    }

    fn begin_at_key(&self, txn: &dyn Transaction, key: &PendingKey) -> PendingIterator {
        let key_bytes = key.to_bytes();
        LmdbIteratorImpl::new_iterator(txn, self.database, Some(&key_bytes), true)
    }

    fn exists(&self, txn: &dyn Transaction, key: &PendingKey) -> bool {
        let iterator = self.begin_at_key(txn, key);
        iterator.current().map(|(k, _)| k == key).unwrap_or(false)
    }

    fn any(&self, txn: &dyn Transaction, account: &Account) -> bool {
        let key = PendingKey::new(*account, BlockHash::zero());
        let iterator = self.begin_at_key(txn, &key);
        iterator
            .current()
            .map(|(k, _)| k.account == *account)
            .unwrap_or(false)
    }

    fn for_each_par(
        &self,
        action: &(dyn Fn(&dyn ReadTransaction, PendingIterator, PendingIterator) + Send + Sync),
    ) {
        parallel_traversal_u512(&|start, end, is_last| {
            let transaction = self.env.tx_begin_read().unwrap();
            let begin_it = self.begin_at_key(&transaction, &start.into());
            let end_it = if !is_last {
                self.begin_at_key(&transaction, &end.into())
            } else {
                self.end()
            };
            action(&transaction, begin_it, end_it);
        });
    }

    fn end(&self) -> PendingIterator {
        LmdbIteratorImpl::null_iterator()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        core::{Amount, Epoch},
        ledger::datastore::lmdb::TestLmdbEnv,
    };

    #[test]
    fn not_found() -> anyhow::Result<()> {
        let env = TestLmdbEnv::new();
        let store = LmdbPendingStore::new(env.env())?;
        let txn = env.tx_begin_read()?;
        let result = store.get(&txn, &test_key());
        assert!(result.is_none());
        assert_eq!(store.exists(&txn, &test_key()), false);
        Ok(())
    }

    #[test]
    fn add_pending() -> anyhow::Result<()> {
        let env = TestLmdbEnv::new();
        let store = LmdbPendingStore::new(env.env())?;
        let mut txn = env.tx_begin_write()?;
        let pending_key = test_key();
        let pending = test_pending_info();
        store.put(&mut txn, &pending_key, &pending);
        let result = store.get(&txn, &pending_key);
        assert_eq!(result, Some(pending));
        assert!(store.exists(&txn, &pending_key));
        Ok(())
    }

    #[test]
    fn delete() -> anyhow::Result<()> {
        let env = TestLmdbEnv::new();
        let store = LmdbPendingStore::new(env.env())?;
        let mut txn = env.tx_begin_write()?;
        let pending_key = test_key();
        let pending = test_pending_info();
        store.put(&mut txn, &pending_key, &pending);
        store.del(&mut txn, &pending_key);
        let result = store.get(&txn, &pending_key);
        assert!(result.is_none());
        Ok(())
    }

    #[test]
    fn iter_empty() -> anyhow::Result<()> {
        let env = TestLmdbEnv::new();
        let store = LmdbPendingStore::new(env.env())?;
        let txn = env.tx_begin_read()?;
        assert!(store.begin(&txn).is_end());
        Ok(())
    }

    #[test]
    fn iter() -> anyhow::Result<()> {
        let env = TestLmdbEnv::new();
        let store = LmdbPendingStore::new(env.env())?;
        let mut txn = env.tx_begin_write()?;
        let pending_key = test_key();
        let pending = test_pending_info();
        store.put(&mut txn, &pending_key, &pending);

        let mut it = store.begin(&txn);
        assert_eq!(it.is_end(), false);
        let (k, v) = it.current().unwrap();
        assert_eq!(k, &pending_key);
        assert_eq!(v, &pending);

        it.next();
        assert!(it.is_end());
        Ok(())
    }

    fn test_key() -> PendingKey {
        PendingKey::new(Account::from(1), BlockHash::from(2))
    }

    fn test_pending_info() -> PendingInfo {
        PendingInfo::new(Account::from(3), Amount::new(4), Epoch::Epoch2)
    }
}
