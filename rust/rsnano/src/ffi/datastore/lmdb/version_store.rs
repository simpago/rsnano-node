use super::TransactionHandle;
use crate::datastore::{lmdb::LmdbVersionStore, VersionStore};
use std::sync::Arc;

pub struct LmdbVersionStoreHandle(Arc<LmdbVersionStore>);

impl LmdbVersionStoreHandle {
    pub fn new(store: Arc<LmdbVersionStore>) -> *mut Self {
        Box::into_raw(Box::new(LmdbVersionStoreHandle(store)))
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_version_store_destroy(handle: *mut LmdbVersionStoreHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_version_store_table_handle(
    handle: *mut LmdbVersionStoreHandle,
) -> u32 {
    (*handle).0.db_handle()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_version_store_open_db(
    handle: *mut LmdbVersionStoreHandle,
    txn: *mut TransactionHandle,
    flags: u32,
) -> bool {
    (*handle).0.open_db((*txn).as_txn(), flags).is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_version_store_put(
    handle: *mut LmdbVersionStoreHandle,
    txn: *mut TransactionHandle,
    version: i32,
) {
    (*handle).0.put((*txn).as_write_txn(), version);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_version_store_get(
    handle: *mut LmdbVersionStoreHandle,
    txn: *mut TransactionHandle,
) -> i32 {
    (*handle).0.get((*txn).as_txn())
}
