use std::sync::{Arc, Mutex};

use crate::{
    datastore::{
        lmdb::{MDB_NOTFOUND, MDB_SUCCESS},
        parallel_traversal, DbIterator, FrontierStore, NullIterator, ReadTransaction, Transaction,
        WriteTransaction,
    },
    Account, BlockHash,
};

use super::{
    assert_success, ensure_success, get_raw_lmdb_txn, mdb_dbi_open, mdb_del, mdb_get, mdb_put,
    LmdbEnv, LmdbIterator, MdbVal,
};

pub struct LmdbFrontierStore {
    env: Arc<LmdbEnv>,
    db_handle: Mutex<u32>,
}

impl LmdbFrontierStore {
    pub fn new(env: Arc<LmdbEnv>) -> Self {
        Self {
            env,
            db_handle: Mutex::new(0),
        }
    }

    pub fn db_handle(&self) -> u32 {
        *self.db_handle.lock().unwrap()
    }

    pub fn open_db(&self, txn: &dyn Transaction, flags: u32) -> anyhow::Result<()> {
        let mut handle = 0;
        let status =
            unsafe { mdb_dbi_open(get_raw_lmdb_txn(txn), "frontiers", flags, &mut handle) };

        let mut guard = self.db_handle.lock().unwrap();
        *guard = handle;

        ensure_success(status)
    }
}

impl FrontierStore for LmdbFrontierStore {
    fn put(&self, txn: &dyn WriteTransaction, hash: &BlockHash, account: &Account) {
        let status = unsafe {
            mdb_put(
                get_raw_lmdb_txn(txn.as_transaction()),
                self.db_handle(),
                &mut MdbVal::from(hash),
                &mut MdbVal::from(account),
                0,
            )
        };
        assert_success(status);
    }

    fn get(&self, txn: &dyn crate::datastore::Transaction, hash: &BlockHash) -> Account {
        let mut value = MdbVal::new();
        let status = unsafe {
            mdb_get(
                get_raw_lmdb_txn(txn),
                self.db_handle(),
                &mut MdbVal::from(hash),
                &mut value,
            )
        };
        assert!(status == MDB_SUCCESS || status == MDB_NOTFOUND);
        if status == MDB_SUCCESS {
            Account::from_slice(value.as_slice()).unwrap_or_default()
        } else {
            Account::new()
        }
    }

    fn del(&self, txn: &dyn WriteTransaction, hash: &BlockHash) {
        let status = unsafe {
            mdb_del(
                get_raw_lmdb_txn(txn.as_transaction()),
                self.db_handle(),
                &mut MdbVal::from(hash),
                None,
            )
        };
        assert_success(status);
    }

    fn begin(
        &self,
        txn: &dyn crate::datastore::Transaction,
    ) -> Box<dyn crate::datastore::DbIterator<BlockHash, Account>> {
        Box::new(LmdbIterator::new(txn, self.db_handle(), None, true))
    }

    fn begin_at_hash(
        &self,
        txn: &dyn crate::datastore::Transaction,
        hash: &BlockHash,
    ) -> Box<dyn crate::datastore::DbIterator<BlockHash, Account>> {
        Box::new(LmdbIterator::new(txn, self.db_handle(), Some(hash), true))
    }

    fn for_each_par(
        &self,
        action: &(dyn Fn(
            &dyn ReadTransaction,
            &mut dyn DbIterator<BlockHash, Account>,
            &mut dyn DbIterator<BlockHash, Account>,
        ) + Send
              + Sync),
    ) {
        parallel_traversal(&|start, end, is_last| {
            let mut transaction = self.env.tx_begin_read();
            let mut begin_it = self.begin_at_hash(&transaction, &start.into());
            let mut end_it = if !is_last {
                self.begin_at_hash(&transaction, &end.into())
            } else {
                self.end()
            };
            action(&mut transaction, begin_it.as_mut(), end_it.as_mut());
        });
    }

    fn end(&self) -> Box<dyn DbIterator<BlockHash, Account>> {
        Box::new(NullIterator::new())
    }
}
