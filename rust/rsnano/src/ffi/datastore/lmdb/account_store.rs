use std::{
    ffi::c_void,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use crate::{
    datastore::{lmdb::LmdbAccountStore, AccountStore},
    ffi::{AccountInfoHandle, VoidPointerCallback},
    Account,
};

use super::{
    iterator::{
        to_lmdb_iterator_handle, ForEachParCallback, ForEachParWrapper, LmdbIteratorHandle,
    },
    TransactionHandle,
};

pub struct LmdbAccountStoreHandle(Arc<LmdbAccountStore>);

impl LmdbAccountStoreHandle {
    pub fn new(store: Arc<LmdbAccountStore>) -> *mut Self {
        Box::into_raw(Box::new(LmdbAccountStoreHandle(store)))
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_account_store_destroy(handle: *mut LmdbAccountStoreHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_account_store_accounts_handle(
    handle: *mut LmdbAccountStoreHandle,
) -> u32 {
    (*handle).0.db_handle()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_account_store_put(
    handle: *mut LmdbAccountStoreHandle,
    txn: *mut TransactionHandle,
    account: *const u8,
    info: *const AccountInfoHandle,
) {
    let account = Account::from_ptr(account);
    let info = (*info).deref();
    (*handle).0.put((*txn).as_write_txn(), &account, info);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_account_store_get(
    handle: *mut LmdbAccountStoreHandle,
    txn: *mut TransactionHandle,
    account: *const u8,
    info: *mut AccountInfoHandle,
) -> bool {
    let account = Account::from_ptr(account);
    let info = (*info).deref_mut();
    match (*handle).0.get((*txn).as_txn(), &account) {
        Some(i) => {
            *info = i;
            true
        }
        None => false,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_account_store_del(
    handle: *mut LmdbAccountStoreHandle,
    txn: *mut TransactionHandle,
    account: *const u8,
) {
    let account = Account::from_ptr(account);
    (*handle).0.del((*txn).as_write_txn(), &account);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_account_store_begin_account(
    handle: *mut LmdbAccountStoreHandle,
    txn: *mut TransactionHandle,
    account: *const u8,
) -> *mut LmdbIteratorHandle {
    let account = Account::from_ptr(account);
    let mut iterator = (*handle).0.begin_account((*txn).as_txn(), &account);
    to_lmdb_iterator_handle(iterator.as_mut())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_account_store_begin(
    handle: *mut LmdbAccountStoreHandle,
    txn: *mut TransactionHandle,
) -> *mut LmdbIteratorHandle {
    let mut iterator = (*handle).0.begin((*txn).as_txn());
    to_lmdb_iterator_handle(iterator.as_mut())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_account_store_rbegin(
    handle: *mut LmdbAccountStoreHandle,
    txn: *mut TransactionHandle,
) -> *mut LmdbIteratorHandle {
    let mut iterator = (*handle).0.rbegin((*txn).as_txn());
    to_lmdb_iterator_handle(iterator.as_mut())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_account_store_count(
    handle: *mut LmdbAccountStoreHandle,
    txn: *mut TransactionHandle,
) -> usize {
    (*handle).0.count((*txn).as_txn())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_account_store_for_each_par(
    handle: *mut LmdbAccountStoreHandle,
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
