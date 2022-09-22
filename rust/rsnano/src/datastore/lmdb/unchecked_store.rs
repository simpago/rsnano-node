use std::sync::{Arc, Mutex};

use crate::{
    datastore::{DbIterator, Transaction, UncheckedStore, WriteTransaction},
    unchecked_info::{UncheckedInfo, UncheckedKey},
    HashOrAccount,
};

use super::{
    assert_success, ensure_success, get_raw_lmdb_txn, mdb_count, mdb_dbi_open, mdb_del, mdb_drop,
    mdb_get, mdb_put, LmdbEnv, LmdbIterator, MdbVal, OwnedMdbVal, MDB_NOTFOUND, MDB_SUCCESS,
};

pub struct LmdbUncheckedStore {
    env: Arc<LmdbEnv>,
    db_handle: Mutex<u32>,
}

impl LmdbUncheckedStore {
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
            unsafe { mdb_dbi_open(get_raw_lmdb_txn(txn), "unchecked", flags, &mut handle) };
        *self.db_handle.lock().unwrap() = handle;
        ensure_success(status)
    }
}

impl UncheckedStore for LmdbUncheckedStore {
    fn clear(&self, txn: &dyn WriteTransaction) {
        let status =
            unsafe { mdb_drop(get_raw_lmdb_txn(txn.as_transaction()), self.db_handle(), 0) };
        assert_success(status);
    }

    fn put(&self, txn: &dyn WriteTransaction, dependency: &HashOrAccount, info: &UncheckedInfo) {
        let key = UncheckedKey {
            previous: dependency.to_block_hash(),
            hash: info
                .block
                .as_ref()
                .unwrap()
                .read()
                .unwrap()
                .as_block()
                .hash(),
        };
        let key_bytes = key.to_bytes();
        let mut value = OwnedMdbVal::from(info);
        let status = unsafe {
            mdb_put(
                get_raw_lmdb_txn(txn.as_transaction()),
                self.db_handle(),
                &mut MdbVal::from_slice(&key_bytes),
                value.as_mdb_val(),
                0,
            )
        };
        assert_success(status);
    }

    fn exists(&self, txn: &dyn Transaction, key: &UncheckedKey) -> bool {
        let mut value = MdbVal::new();
        let key_bytes = key.to_bytes();
        let status = unsafe {
            mdb_get(
                get_raw_lmdb_txn(txn),
                self.db_handle(),
                &mut MdbVal::from_slice(&key_bytes),
                &mut value,
            )
        };
        assert!(status == MDB_SUCCESS || status == MDB_NOTFOUND);
        status == MDB_SUCCESS
    }

    fn del(&self, txn: &dyn WriteTransaction, key: &UncheckedKey) {
        let key_bytes = key.to_bytes();
        let status = unsafe {
            mdb_del(
                get_raw_lmdb_txn(txn.as_transaction()),
                self.db_handle(),
                &mut MdbVal::from_slice(&key_bytes),
                None,
            )
        };
        assert_success(status);
    }

    fn begin(&self, txn: &dyn Transaction) -> Box<dyn DbIterator<UncheckedKey, UncheckedInfo>> {
        Box::new(LmdbIterator::new(txn, self.db_handle(), None, true))
    }

    fn lower_bound(
        &self,
        txn: &dyn Transaction,
        key: &UncheckedKey,
    ) -> Box<dyn DbIterator<UncheckedKey, UncheckedInfo>> {
        Box::new(LmdbIterator::new(txn, self.db_handle(), Some(key), true))
    }

    fn count(&self, txn: &dyn Transaction) -> usize {
        unsafe { mdb_count(get_raw_lmdb_txn(txn), self.db_handle()) }
    }
}
