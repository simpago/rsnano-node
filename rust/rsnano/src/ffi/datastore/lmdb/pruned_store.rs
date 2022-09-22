use std::{ffi::c_void, sync::Arc};

use crate::{
    datastore::{lmdb::LmdbPrunedStore, PrunedStore},
    ffi::{copy_hash_bytes, VoidPointerCallback},
    BlockHash,
};

use super::{
    iterator::{
        to_lmdb_iterator_handle, ForEachParCallback, ForEachParWrapper, LmdbIteratorHandle,
    },
    TransactionHandle,
};

pub struct LmdbPrunedStoreHandle(Arc<LmdbPrunedStore>);

impl LmdbPrunedStoreHandle {
    pub fn new(store: Arc<LmdbPrunedStore>) -> *mut Self {
        Box::into_raw(Box::new(LmdbPrunedStoreHandle(store)))
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_pruned_store_destroy(handle: *mut LmdbPrunedStoreHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_pruned_store_table_handle(
    handle: *mut LmdbPrunedStoreHandle,
) -> u32 {
    (*handle).0.db_handle()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_pruned_store_put(
    handle: *mut LmdbPrunedStoreHandle,
    txn: *mut TransactionHandle,
    hash: *const u8,
) {
    (*handle)
        .0
        .put((*txn).as_write_txn(), &BlockHash::from_ptr(hash));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_pruned_store_del(
    handle: *mut LmdbPrunedStoreHandle,
    txn: *mut TransactionHandle,
    hash: *const u8,
) {
    (*handle)
        .0
        .del((*txn).as_write_txn(), &BlockHash::from_ptr(hash));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_pruned_store_exists(
    handle: *mut LmdbPrunedStoreHandle,
    txn: *mut TransactionHandle,
    hash: *const u8,
) -> bool {
    (*handle)
        .0
        .exists((*txn).as_txn(), &BlockHash::from_ptr(hash))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_pruned_store_begin(
    handle: *mut LmdbPrunedStoreHandle,
    txn: *mut TransactionHandle,
) -> *mut LmdbIteratorHandle {
    let mut iterator = (*handle).0.begin((*txn).as_txn());
    to_lmdb_iterator_handle(iterator.as_mut())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_pruned_store_begin_at_hash(
    handle: *mut LmdbPrunedStoreHandle,
    txn: *mut TransactionHandle,
    hash: *const u8,
) -> *mut LmdbIteratorHandle {
    let mut iterator = (*handle)
        .0
        .begin_at_hash((*txn).as_txn(), &BlockHash::from_ptr(hash));
    to_lmdb_iterator_handle(iterator.as_mut())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_pruned_store_random(
    handle: *mut LmdbPrunedStoreHandle,
    txn: *mut TransactionHandle,
    hash: *mut u8,
) {
    let random = (*handle).0.random((*txn).as_txn());
    copy_hash_bytes(random, hash);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_pruned_store_count(
    handle: *mut LmdbPrunedStoreHandle,
    txn: *mut TransactionHandle,
) -> usize {
    (*handle).0.count((*txn).as_txn())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_pruned_store_clear(
    handle: *mut LmdbPrunedStoreHandle,
    txn: *mut TransactionHandle,
) {
    (*handle).0.clear((*txn).as_write_txn())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_pruned_store_for_each_par(
    handle: *mut LmdbPrunedStoreHandle,
    action: ForEachParCallback,
    context: *mut c_void,
    delete_context: VoidPointerCallback,
) {
    let wrapper = ForEachParWrapper {
        action,
        context,
        delete_context,
    };
    (*handle)
        .0
        .for_each_par(&|txn, begin, end| wrapper.execute(txn, begin, end));
}
