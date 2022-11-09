use std::sync::Arc;

use lmdb::{Database, DatabaseFlags, WriteFlags};

use crate::{
    core::Amount,
    ledger::datastore::{
        online_weight_store::OnlineWeightIterator, OnlineWeightStore, Transaction, WriteTransaction,
    },
};

use super::{as_write_txn, count, LmdbEnv, LmdbIteratorImpl};

pub struct LmdbOnlineWeightStore {
    env: Arc<LmdbEnv>,
    database: Database,
}

impl LmdbOnlineWeightStore {
    pub fn new(env: Arc<LmdbEnv>) -> anyhow::Result<Self> {
        let database = env
            .environment
            .create_db(Some("online_weight"), DatabaseFlags::empty())?;
        Ok(Self { env, database })
    }

    pub fn database(&self) -> Database {
        self.database
    }
}

impl OnlineWeightStore for LmdbOnlineWeightStore {
    fn put(&self, txn: &mut dyn WriteTransaction, time: u64, amount: &Amount) {
        let time_bytes = time.to_be_bytes();
        let amount_bytes = amount.to_be_bytes();
        as_write_txn(txn)
            .put(
                self.database,
                &time_bytes,
                &amount_bytes,
                WriteFlags::empty(),
            )
            .unwrap();
    }

    fn del(&self, txn: &mut dyn WriteTransaction, time: u64) {
        let time_bytes = time.to_be_bytes();
        as_write_txn(txn)
            .del(self.database, &time_bytes, None)
            .unwrap();
    }

    fn begin(&self, txn: &dyn Transaction) -> OnlineWeightIterator {
        LmdbIteratorImpl::new_iterator(txn, self.database, None, true)
    }

    fn rbegin(&self, txn: &dyn Transaction) -> OnlineWeightIterator {
        LmdbIteratorImpl::new_iterator(txn, self.database, None, false)
    }

    fn count(&self, txn: &dyn Transaction) -> usize {
        count(txn, self.database)
    }

    fn clear(&self, txn: &mut dyn WriteTransaction) {
        as_write_txn(txn).clear_db(self.database).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::datastore::lmdb::TestLmdbEnv;

    #[test]
    fn empty_store() -> anyhow::Result<()> {
        let env = TestLmdbEnv::new();
        let store = LmdbOnlineWeightStore::new(env.env())?;
        let txn = env.tx_begin_read()?;
        assert_eq!(store.count(&txn), 0);
        assert!(store.begin(&txn).is_end());
        assert!(store.rbegin(&txn).is_end());
        Ok(())
    }

    #[test]
    fn add_one() -> anyhow::Result<()> {
        let env = TestLmdbEnv::new();
        let store = LmdbOnlineWeightStore::new(env.env())?;
        let mut txn = env.tx_begin_write()?;

        let time = 1;
        let amount = Amount::new(2);
        store.put(&mut txn, time, &amount);

        assert_eq!(store.count(&txn), 1);
        assert_eq!(store.begin(&txn).current(), Some((&time, &amount)));
        assert_eq!(store.rbegin(&txn).current(), Some((&time, &amount)));
        Ok(())
    }

    #[test]
    fn add_two() -> anyhow::Result<()> {
        let env = TestLmdbEnv::new();
        let store = LmdbOnlineWeightStore::new(env.env())?;
        let mut txn = env.tx_begin_write()?;

        let time1 = 1;
        let time2 = 2;
        let amount1 = Amount::new(3);
        let amount2 = Amount::new(4);
        store.put(&mut txn, time1, &amount1);
        store.put(&mut txn, time2, &amount2);

        assert_eq!(store.count(&txn), 2);
        assert_eq!(store.begin(&txn).current(), Some((&time1, &amount1)));
        assert_eq!(store.rbegin(&txn).current(), Some((&time2, &amount2)));
        Ok(())
    }

    #[test]
    fn delete() -> anyhow::Result<()> {
        let env = TestLmdbEnv::new();
        let store = LmdbOnlineWeightStore::new(env.env())?;
        let mut txn = env.tx_begin_write()?;

        let time1 = 1;
        let time2 = 2;
        let amount1 = Amount::new(3);
        let amount2 = Amount::new(4);
        store.put(&mut txn, time1, &amount1);
        store.put(&mut txn, time2, &amount2);

        store.del(&mut txn, time1);

        assert_eq!(store.count(&txn), 1);
        assert_eq!(store.begin(&txn).current(), Some((&time2, &amount2)));
        Ok(())
    }
}
