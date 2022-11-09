use std::{ffi::c_void, sync::Arc};

use crate::{
    core::{BlockHash, QualifiedRoot, Root},
    ffi::VoidPointerCallback,
    ledger::datastore::{lmdb::LmdbFinalVoteStore, FinalVoteStore},
};

use super::{
    iterator::{ForEachParCallback, ForEachParWrapper, LmdbIteratorHandle},
    TransactionHandle,
};

pub struct LmdbFinalVoteStoreHandle(Arc<LmdbFinalVoteStore>);

impl LmdbFinalVoteStoreHandle {
    pub fn new(store: Arc<LmdbFinalVoteStore>) -> *mut Self {
        Box::into_raw(Box::new(LmdbFinalVoteStoreHandle(store)))
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_final_vote_store_destroy(handle: *mut LmdbFinalVoteStoreHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_final_vote_store_put(
    handle: *mut LmdbFinalVoteStoreHandle,
    txn: *mut TransactionHandle,
    root: *const u8,
    hash: *const u8,
) -> bool {
    (*handle).0.put(
        (*txn).as_write_txn(),
        &QualifiedRoot::from_ptr(root),
        &BlockHash::from_ptr(hash),
    )
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_final_vote_store_begin(
    handle: *mut LmdbFinalVoteStoreHandle,
    txn: *mut TransactionHandle,
) -> *mut LmdbIteratorHandle {
    let iterator = (*handle).0.begin((*txn).as_txn());
    LmdbIteratorHandle::new2(iterator)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_final_vote_store_begin_at_root(
    handle: *mut LmdbFinalVoteStoreHandle,
    txn: *mut TransactionHandle,
    root: *const u8,
) -> *mut LmdbIteratorHandle {
    let root = QualifiedRoot::from_ptr(root);
    let iterator = (*handle).0.begin_at_root((*txn).as_txn(), &root);
    LmdbIteratorHandle::new2(iterator)
}

#[repr(C)]
pub struct BlockHashArrayDto {
    pub data: *const u8,
    pub count: usize,
    pub raw_data: *mut c_void,
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_final_vote_store_get(
    handle: *mut LmdbFinalVoteStoreHandle,
    txn: *mut TransactionHandle,
    root: *const u8,
    result: *mut BlockHashArrayDto,
) {
    let hashes = (*handle).0.get((*txn).as_txn(), Root::from_ptr(root));
    let mut bytes = Box::new(Vec::with_capacity(hashes.len() * 32));
    for h in &hashes {
        for &b in h.as_bytes() {
            bytes.push(b);
        }
    }
    (*result).count = bytes.len();
    (*result).data = bytes.as_ptr();
    (*result).raw_data = Box::into_raw(bytes) as *mut c_void;
}

#[no_mangle]
pub unsafe extern "C" fn rsn_block_hash_array_destroy(data: *mut BlockHashArrayDto) {
    let v = (*data).raw_data as *mut Vec<u8>;
    drop(Box::from_raw(v))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_final_vote_store_del(
    handle: *mut LmdbFinalVoteStoreHandle,
    txn: *mut TransactionHandle,
    root: *const u8,
) {
    (*handle)
        .0
        .del((*txn).as_write_txn(), &Root::from_ptr(root));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_final_vote_store_count(
    handle: *mut LmdbFinalVoteStoreHandle,
    txn: *mut TransactionHandle,
) -> usize {
    (*handle).0.count((*txn).as_txn())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_final_vote_store_clear(
    handle: *mut LmdbFinalVoteStoreHandle,
    txn: *mut TransactionHandle,
) {
    (*handle).0.clear((*txn).as_write_txn());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_final_vote_store_for_each_par(
    handle: *mut LmdbFinalVoteStoreHandle,
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
