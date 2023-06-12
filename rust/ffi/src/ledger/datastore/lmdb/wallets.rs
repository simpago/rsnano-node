use std::ffi::{c_char, CStr};

use rsnano_core::BlockHash;
use rsnano_store_lmdb::LmdbWallets;

use crate::{copy_hash_bytes, U256ArrayDto};

use super::{store::LmdbStoreHandle, TransactionHandle};

pub struct LmdbWalletsHandle(LmdbWallets);

#[no_mangle]
pub extern "C" fn rsn_lmdb_wallets_create() -> *mut LmdbWalletsHandle {
    Box::into_raw(Box::new(LmdbWalletsHandle(LmdbWallets::new())))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallets_destroy(handle: *mut LmdbWalletsHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallets_init(
    handle: *mut LmdbWalletsHandle,
    txn: *mut TransactionHandle,
    store: *mut LmdbStoreHandle,
) -> bool {
    (*handle)
        .0
        .initialize((*txn).as_write_txn(), &(*store).env)
        .is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallets_get_wallet_ids(
    handle: *mut LmdbWalletsHandle,
    txn: *mut TransactionHandle,
    result: *mut U256ArrayDto,
) {
    let wallet_ids = (*handle).0.get_wallet_ids((*txn).as_txn());
    let data = wallet_ids.iter().map(|i| *i.as_bytes()).collect();
    (*result).initialize(data)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallets_get_block_hash(
    handle: *mut LmdbWalletsHandle,
    txn: *mut TransactionHandle,
    id: *const c_char,
    hash: *mut u8,
) -> bool {
    let id = CStr::from_ptr(id).to_str().unwrap();
    match (*handle).0.get_block_hash((*txn).as_txn(), id) {
        Ok(Some(h)) => {
            copy_hash_bytes(h, hash);
            true
        }
        Ok(None) => {
            copy_hash_bytes(BlockHash::zero(), hash);
            true
        }
        Err(_) => false,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallets_set_block_hash(
    handle: *mut LmdbWalletsHandle,
    txn: *mut TransactionHandle,
    id: *const c_char,
    hash: *const u8,
) -> bool {
    let id = CStr::from_ptr(id).to_str().unwrap();
    (*handle)
        .0
        .set_block_hash((*txn).as_write_txn(), id, &BlockHash::from_ptr(hash))
        .is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_wallets_clear_send_ids(
    handle: *mut LmdbWalletsHandle,
    txn: *mut TransactionHandle,
) {
    (*handle).0.clear_send_ids((*txn).as_write_txn())
}
