use lmdb::{Database, DatabaseFlags, WriteFlags};
use std::sync::Arc;

use crate::{
    core::EndpointKey,
    ledger::datastore::{peer_store::PeerIterator, PeerStore, Transaction, WriteTransaction},
};

use super::{as_write_txn, count, exists, LmdbEnv, LmdbIteratorImpl};

pub struct LmdbPeerStore {
    env: Arc<LmdbEnv>,
    database: Database,
}

impl LmdbPeerStore {
    pub fn new(env: Arc<LmdbEnv>) -> anyhow::Result<Self> {
        let database = env
            .environment
            .create_db(Some("peers"), DatabaseFlags::empty())?;

        Ok(Self { env, database })
    }

    pub fn database(&self) -> Database {
        self.database
    }
}

impl PeerStore for LmdbPeerStore {
    fn put(&self, txn: &mut dyn WriteTransaction, endpoint: &EndpointKey) {
        as_write_txn(txn)
            .put(
                self.database,
                &endpoint.to_bytes(),
                &[0; 0],
                WriteFlags::empty(),
            )
            .unwrap();
    }

    fn del(&self, txn: &mut dyn WriteTransaction, endpoint: &EndpointKey) {
        as_write_txn(txn)
            .del(self.database, &endpoint.to_bytes(), None)
            .unwrap();
    }

    fn exists(&self, txn: &dyn Transaction, endpoint: &EndpointKey) -> bool {
        exists(txn, self.database, &endpoint.to_bytes())
    }

    fn count(&self, txn: &dyn Transaction) -> usize {
        count(txn, self.database)
    }

    fn clear(&self, txn: &mut dyn WriteTransaction) {
        as_write_txn(txn).clear_db(self.database).unwrap();
    }

    fn begin(&self, txn: &dyn Transaction) -> PeerIterator {
        LmdbIteratorImpl::new_iterator(txn, self.database, None, true)
    }
}

#[cfg(test)]
mod tests {
    use crate::{core::NoValue, ledger::datastore::lmdb::TestLmdbEnv};

    use super::*;

    #[test]
    fn empty_store() -> anyhow::Result<()> {
        let env = TestLmdbEnv::new();
        let store = LmdbPeerStore::new(env.env())?;
        let txn = env.tx_begin_read()?;
        assert_eq!(store.count(&txn), 0);
        assert_eq!(store.exists(&txn, &test_endpoint_key()), false);
        assert!(store.begin(&txn).is_end());
        Ok(())
    }

    #[test]
    fn add_one_endpoint() -> anyhow::Result<()> {
        let env = TestLmdbEnv::new();
        let store = LmdbPeerStore::new(env.env())?;
        let mut txn = env.tx_begin_write()?;

        let key = test_endpoint_key();
        store.put(&mut txn, &key);

        assert_eq!(store.count(&txn), 1);
        assert_eq!(store.exists(&txn, &key), true);
        assert_eq!(store.begin(&txn).current(), Some((&key, &NoValue {})));
        Ok(())
    }

    #[test]
    fn add_two_endpoints() -> anyhow::Result<()> {
        let env = TestLmdbEnv::new();
        let store = LmdbPeerStore::new(env.env())?;
        let mut txn = env.tx_begin_write()?;

        let key1 = test_endpoint_key();
        let key2 = EndpointKey::new([2; 16], 123);
        store.put(&mut txn, &key1);
        store.put(&mut txn, &key2);

        assert_eq!(store.count(&txn), 2);
        assert_eq!(store.exists(&txn, &key1), true);
        assert_eq!(store.exists(&txn, &key2), true);
        Ok(())
    }

    #[test]
    fn delete() -> anyhow::Result<()> {
        let env = TestLmdbEnv::new();
        let store = LmdbPeerStore::new(env.env())?;
        let mut txn = env.tx_begin_write()?;

        let key1 = test_endpoint_key();
        let key2 = EndpointKey::new([2; 16], 123);
        store.put(&mut txn, &key1);
        store.put(&mut txn, &key2);

        store.del(&mut txn, &key1);

        assert_eq!(store.count(&txn), 1);
        assert_eq!(store.exists(&txn, &key1), false);
        assert_eq!(store.exists(&txn, &key2), true);
        Ok(())
    }

    fn test_endpoint_key() -> EndpointKey {
        EndpointKey::new([1; 16], 123)
    }
}
