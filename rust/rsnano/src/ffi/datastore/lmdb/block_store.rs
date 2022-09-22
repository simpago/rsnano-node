use std::{
    ffi::c_void,
    ptr, slice,
    sync::{Arc, RwLock},
};

use crate::{
    datastore::{lmdb::LmdbBlockStore, BlockStore},
    ffi::{
        copy_account_bytes, copy_amount_bytes, copy_hash_bytes, BlockHandle, VoidPointerCallback,
    },
    BlockHash,
};

use super::{
    iterator::{
        to_lmdb_iterator_handle, ForEachParCallback, ForEachParWrapper, LmdbIteratorHandle,
    },
    TransactionHandle,
};

pub struct LmdbBlockStoreHandle(Arc<LmdbBlockStore>);

impl LmdbBlockStoreHandle {
    pub fn new(store: Arc<LmdbBlockStore>) -> *mut Self {
        Box::into_raw(Box::new(LmdbBlockStoreHandle(store)))
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_block_store_destroy(handle: *mut LmdbBlockStoreHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_block_store_blocks_handle(
    handle: *mut LmdbBlockStoreHandle,
) -> u32 {
    (*handle).0.db_handle()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_block_store_raw_put(
    handle: *mut LmdbBlockStoreHandle,
    txn: *mut TransactionHandle,
    data: *const u8,
    len: usize,
    hash: *const u8,
) {
    let txn = (*txn).as_write_txn();
    let data = slice::from_raw_parts(data, len);
    let hash = BlockHash::from_ptr(hash);
    (*handle).0.raw_put(txn, data, &hash);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_block_store_put(
    handle: *mut LmdbBlockStoreHandle,
    txn: *mut TransactionHandle,
    hash: *const u8,
    block: *mut BlockHandle,
) {
    (*handle).0.put(
        (*txn).as_write_txn(),
        &BlockHash::from_ptr(hash),
        (*block).block.read().unwrap().as_block(),
    );
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_block_store_exists(
    handle: *mut LmdbBlockStoreHandle,
    txn: *mut TransactionHandle,
    hash: *const u8,
) -> bool {
    (*handle)
        .0
        .exists((*txn).as_txn(), &BlockHash::from_ptr(hash))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_block_store_successor(
    handle: *mut LmdbBlockStoreHandle,
    txn: *mut TransactionHandle,
    hash: *const u8,
    result: *mut u8,
) {
    let successor = (*handle)
        .0
        .successor((*txn).as_txn(), &BlockHash::from_ptr(hash));
    copy_hash_bytes(successor, result);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_block_store_successor_clear(
    handle: *mut LmdbBlockStoreHandle,
    txn: *mut TransactionHandle,
    hash: *const u8,
) {
    (*handle)
        .0
        .successor_clear((*txn).as_write_txn(), &BlockHash::from_ptr(hash));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_block_store_get(
    handle: *mut LmdbBlockStoreHandle,
    txn: *mut TransactionHandle,
    hash: *const u8,
) -> *mut BlockHandle {
    match (*handle).0.get((*txn).as_txn(), &BlockHash::from_ptr(hash)) {
        Some(block) => Box::into_raw(Box::new(BlockHandle::new(Arc::new(RwLock::new(block))))),
        None => ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_block_store_get_no_sideband(
    handle: *mut LmdbBlockStoreHandle,
    txn: *mut TransactionHandle,
    hash: *const u8,
) -> *mut BlockHandle {
    match (*handle)
        .0
        .get_no_sideband((*txn).as_txn(), &BlockHash::from_ptr(hash))
    {
        Some(block) => Box::into_raw(Box::new(BlockHandle::new(Arc::new(RwLock::new(block))))),
        None => ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_block_store_del(
    handle: *mut LmdbBlockStoreHandle,
    txn: *mut TransactionHandle,
    hash: *const u8,
) {
    (*handle)
        .0
        .del((*txn).as_write_txn(), &BlockHash::from_ptr(hash));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_block_store_count(
    handle: *mut LmdbBlockStoreHandle,
    txn: *mut TransactionHandle,
) -> u64 {
    (*handle).0.count((*txn).as_txn()) as u64
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_block_store_account_calculated(
    handle: *mut LmdbBlockStoreHandle,
    block: *const BlockHandle,
    result: *mut u8,
) {
    let account = (*handle)
        .0
        .account_calculated((*block).block.read().unwrap().as_block());
    copy_account_bytes(account, result);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_block_store_account(
    handle: *mut LmdbBlockStoreHandle,
    txn: *mut TransactionHandle,
    hash: *const u8,
    result: *mut u8,
) {
    let account = (*handle)
        .0
        .account((*txn).as_txn(), &BlockHash::from_ptr(hash));
    copy_account_bytes(account, result);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_block_store_begin(
    handle: *mut LmdbBlockStoreHandle,
    txn: *mut TransactionHandle,
) -> *mut LmdbIteratorHandle {
    let mut iterator = (*handle).0.begin((*txn).as_txn());
    to_lmdb_iterator_handle(iterator.as_mut())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_block_store_begin_at_hash(
    handle: *mut LmdbBlockStoreHandle,
    txn: *mut TransactionHandle,
    hash: *const u8,
) -> *mut LmdbIteratorHandle {
    let hash = BlockHash::from_ptr(hash);
    let mut iterator = (*handle).0.begin_at_hash((*txn).as_txn(), &hash);
    to_lmdb_iterator_handle(iterator.as_mut())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_block_store_random(
    handle: *mut LmdbBlockStoreHandle,
    txn: *mut TransactionHandle,
) -> *mut BlockHandle {
    match (*handle).0.random((*txn).as_txn()) {
        Some(block) => Box::into_raw(Box::new(BlockHandle::new(Arc::new(RwLock::new(block))))),
        None => ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_block_store_balance(
    handle: *mut LmdbBlockStoreHandle,
    txn: *mut TransactionHandle,
    hash: *const u8,
    balance: *mut u8,
) {
    let result = (*handle)
        .0
        .balance((*txn).as_txn(), &BlockHash::from_ptr(hash));
    copy_amount_bytes(result, balance);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_block_store_balance_calculated(
    handle: *mut LmdbBlockStoreHandle,
    block: *const BlockHandle,
    balance: *mut u8,
) {
    let result = (*handle)
        .0
        .balance_calculated(&(*block).block.read().unwrap());
    copy_amount_bytes(result, balance);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_block_store_version(
    handle: *mut LmdbBlockStoreHandle,
    txn: *mut TransactionHandle,
    hash: *const u8,
) -> u8 {
    (*handle)
        .0
        .version((*txn).as_txn(), &BlockHash::from_ptr(hash)) as u8
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_block_store_for_each_par(
    handle: *mut LmdbBlockStoreHandle,
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

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_block_store_account_height(
    handle: *mut LmdbBlockStoreHandle,
    txn: *mut TransactionHandle,
    hash: *const u8,
) -> u64 {
    (*handle)
        .0
        .account_height((*txn).as_txn(), &BlockHash::from_ptr(hash))
}
