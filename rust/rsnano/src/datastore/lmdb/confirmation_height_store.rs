use std::sync::{Arc, Mutex};

use crate::{
    datastore::{
        lmdb::{MDB_NOTFOUND, MDB_SUCCESS},
        parallel_traversal, ConfirmationHeightStore, DbIterator, NullIterator, ReadTransaction,
        Transaction, WriteTransaction,
    },
    utils::Deserialize,
    Account, ConfirmationHeightInfo,
};

use super::{
    assert_success, ensure_success, exists, get_raw_lmdb_txn, mdb_count, mdb_dbi_open, mdb_del,
    mdb_drop, mdb_get, mdb_put, LmdbEnv, LmdbIterator, MdbVal, OwnedMdbVal,
};

pub struct LmdbConfirmationHeightStore {
    env: Arc<LmdbEnv>,
    db_handle: Mutex<u32>,
}

impl LmdbConfirmationHeightStore {
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
        let status = unsafe {
            mdb_dbi_open(
                get_raw_lmdb_txn(txn),
                "confirmation_height",
                flags,
                &mut handle,
            )
        };
        *self.db_handle.lock().unwrap() = handle;
        ensure_success(status)
    }
}

impl ConfirmationHeightStore for LmdbConfirmationHeightStore {
    fn put(
        &self,
        txn: &dyn WriteTransaction,
        account: &crate::Account,
        info: &ConfirmationHeightInfo,
    ) {
        let mut key = MdbVal::from_slice(account.as_bytes());
        let mut value = OwnedMdbVal::from(info);
        let status = unsafe {
            mdb_put(
                get_raw_lmdb_txn(txn.as_transaction()),
                self.db_handle(),
                &mut key,
                value.as_mdb_val(),
                0,
            )
        };
        assert_success(status);
    }

    fn get(
        &self,
        txn: &dyn crate::datastore::Transaction,
        account: &crate::Account,
    ) -> Option<ConfirmationHeightInfo> {
        let mut key = MdbVal::from(account);
        let mut data = MdbVal::new();
        let status =
            unsafe { mdb_get(get_raw_lmdb_txn(txn), self.db_handle(), &mut key, &mut data) };
        assert!(status == MDB_SUCCESS || status == MDB_NOTFOUND);

        if status == MDB_SUCCESS {
            let mut stream = data.as_stream();
            ConfirmationHeightInfo::deserialize(&mut stream).ok()
        } else {
            None
        }
    }

    fn exists(&self, txn: &dyn Transaction, account: &Account) -> bool {
        exists(txn, self.db_handle(), &mut MdbVal::from(account))
    }

    fn del(&self, txn: &dyn Transaction, account: &Account) {
        let status = unsafe {
            mdb_del(
                get_raw_lmdb_txn(txn),
                self.db_handle(),
                &mut MdbVal::from(account),
                None,
            )
        };
        assert_success(status);
    }

    fn count(&self, txn: &dyn Transaction) -> usize {
        unsafe { mdb_count(get_raw_lmdb_txn(txn), self.db_handle()) }
    }

    fn clear(&self, txn: &dyn WriteTransaction) {
        unsafe { mdb_drop(get_raw_lmdb_txn(txn.as_transaction()), self.db_handle(), 0) };
    }

    fn begin(&self, txn: &dyn Transaction) -> Box<dyn DbIterator<Account, ConfirmationHeightInfo>> {
        Box::new(LmdbIterator::new(txn, self.db_handle(), None, true))
    }

    fn begin_at_account(
        &self,
        txn: &dyn Transaction,
        account: &Account,
    ) -> Box<dyn DbIterator<Account, ConfirmationHeightInfo>> {
        Box::new(LmdbIterator::new(
            txn,
            self.db_handle(),
            Some(account),
            true,
        ))
    }

    fn end(&self) -> Box<dyn DbIterator<Account, ConfirmationHeightInfo>> {
        Box::new(NullIterator::new())
    }

    fn for_each_par(
        &self,
        action: &(dyn Fn(
            &dyn ReadTransaction,
            &mut dyn DbIterator<Account, ConfirmationHeightInfo>,
            &mut dyn DbIterator<Account, ConfirmationHeightInfo>,
        ) + Send
              + Sync),
    ) {
        parallel_traversal(&|start, end, is_last| {
            let mut transaction = self.env.tx_begin_read();
            let mut begin_it = self.begin_at_account(&transaction, &start.into());
            let mut end_it = if !is_last {
                self.begin_at_account(&transaction, &end.into())
            } else {
                self.end()
            };
            action(&mut transaction, begin_it.as_mut(), end_it.as_mut());
        });
    }
}
