use std::{ffi::c_void, sync::Arc};

use crate::{
    core::{Account, ConfirmationHeightInfo},
    ffi::{ConfirmationHeightInfoDto, VoidPointerCallback},
    ledger::datastore::{lmdb::LmdbConfirmationHeightStore, ConfirmationHeightStore},
};

use super::{
    iterator::{ForEachParCallback, ForEachParWrapper, LmdbIteratorHandle},
    TransactionHandle,
};

pub struct LmdbConfirmationHeightStoreHandle(Arc<LmdbConfirmationHeightStore>);

impl LmdbConfirmationHeightStoreHandle {
    pub fn new(store: Arc<LmdbConfirmationHeightStore>) -> *mut Self {
        Box::into_raw(Box::new(LmdbConfirmationHeightStoreHandle(store)))
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_confirmation_height_store_destroy(
    handle: *mut LmdbConfirmationHeightStoreHandle,
) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_confirmation_height_store_put(
    handle: *mut LmdbConfirmationHeightStoreHandle,
    txn: *mut TransactionHandle,
    account: *const u8,
    info: *const ConfirmationHeightInfoDto,
) {
    (*handle).0.put(
        (*txn).as_write_txn(),
        &Account::from_ptr(account),
        &ConfirmationHeightInfo::from(&*info),
    );
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_confirmation_height_store_get(
    handle: *mut LmdbConfirmationHeightStoreHandle,
    txn: *mut TransactionHandle,
    account: *const u8,
    info: *mut ConfirmationHeightInfoDto,
) -> bool {
    let result = (*handle)
        .0
        .get((*txn).as_txn(), &Account::from_ptr(account));

    match result {
        Some(i) => {
            (*info) = i.into();
            true
        }
        None => {
            *info = ConfirmationHeightInfo::default().into();
            false
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_confirmation_height_store_exists(
    handle: *mut LmdbConfirmationHeightStoreHandle,
    txn: *mut TransactionHandle,
    account: *const u8,
) -> bool {
    (*handle)
        .0
        .exists((*txn).as_txn(), &Account::from_ptr(account))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_confirmation_height_store_del(
    handle: *mut LmdbConfirmationHeightStoreHandle,
    txn: *mut TransactionHandle,
    account: *const u8,
) {
    (*handle)
        .0
        .del((*txn).as_write_txn(), &Account::from_ptr(account))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_confirmation_height_store_count(
    handle: *mut LmdbConfirmationHeightStoreHandle,
    txn: *mut TransactionHandle,
) -> u64 {
    (*handle).0.count((*txn).as_txn()) as u64
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_confirmation_height_store_clear(
    handle: *mut LmdbConfirmationHeightStoreHandle,
    txn: *mut TransactionHandle,
) {
    (*handle).0.clear((*txn).as_write_txn());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_confirmation_height_store_begin(
    handle: *mut LmdbConfirmationHeightStoreHandle,
    txn: *mut TransactionHandle,
) -> *mut LmdbIteratorHandle {
    let iterator = (*handle).0.begin((*txn).as_txn());
    LmdbIteratorHandle::new2(iterator)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_confirmation_height_store_begin_at_account(
    handle: *mut LmdbConfirmationHeightStoreHandle,
    txn: *mut TransactionHandle,
    account: *const u8,
) -> *mut LmdbIteratorHandle {
    let iterator = (*handle)
        .0
        .begin_at_account((*txn).as_txn(), &Account::from_ptr(account));
    LmdbIteratorHandle::new2(iterator)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_confirmation_height_store_for_each_par(
    handle: *mut LmdbConfirmationHeightStoreHandle,
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
