use num::FromPrimitive;
use std::{ffi::c_void, sync::Arc};

use crate::{
    core::{Account, Amount, BlockHash, Epoch, PendingInfo, PendingKey},
    ffi::VoidPointerCallback,
    ledger::datastore::{lmdb::LmdbPendingStore, PendingStore},
};

use super::{
    iterator::{ForEachParCallback, ForEachParWrapper, LmdbIteratorHandle},
    TransactionHandle,
};

pub struct LmdbPendingStoreHandle(Arc<LmdbPendingStore>);

impl LmdbPendingStoreHandle {
    pub fn new(store: Arc<LmdbPendingStore>) -> *mut Self {
        Box::into_raw(Box::new(LmdbPendingStoreHandle(store)))
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_pending_store_destroy(handle: *mut LmdbPendingStoreHandle) {
    drop(Box::from_raw(handle))
}

#[repr(C)]
pub struct PendingKeyDto {
    pub account: [u8; 32],
    pub hash: [u8; 32],
}

#[repr(C)]
pub struct PendingInfoDto {
    pub source: [u8; 32],
    pub amount: [u8; 16],
    pub epoch: u8,
}

impl From<&PendingKeyDto> for PendingKey {
    fn from(dto: &PendingKeyDto) -> Self {
        Self {
            account: Account::from_bytes(dto.account),
            hash: BlockHash::from_bytes(dto.hash),
        }
    }
}

impl From<&PendingInfoDto> for PendingInfo {
    fn from(dto: &PendingInfoDto) -> Self {
        Self {
            source: Account::from_bytes(dto.source),
            amount: Amount::from_be_bytes(dto.amount),
            epoch: FromPrimitive::from_u8(dto.epoch).unwrap_or(Epoch::Invalid),
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_pending_store_put(
    handle: *mut LmdbPendingStoreHandle,
    txn: *mut TransactionHandle,
    key: *const PendingKeyDto,
    pending: *const PendingInfoDto,
) {
    (*handle).0.put(
        (*txn).as_write_txn(),
        &PendingKey::from(&*key),
        &PendingInfo::from(&*pending),
    );
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_pending_store_del(
    handle: *mut LmdbPendingStoreHandle,
    txn: *mut TransactionHandle,
    key: *const PendingKeyDto,
) {
    (*handle)
        .0
        .del((*txn).as_write_txn(), &PendingKey::from(&*key));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_pending_store_get(
    handle: *mut LmdbPendingStoreHandle,
    txn: *mut TransactionHandle,
    key: *const PendingKeyDto,
    pending: *mut PendingInfoDto,
) -> bool {
    match (*handle).0.get((*txn).as_txn(), &PendingKey::from(&*key)) {
        Some(p) => {
            (*pending).source = *p.source.as_bytes();
            (*pending).amount = p.amount.to_be_bytes();
            (*pending).epoch = p.epoch as u8;
            false
        }
        None => true,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_pending_store_begin(
    handle: *mut LmdbPendingStoreHandle,
    txn: *mut TransactionHandle,
) -> *mut LmdbIteratorHandle {
    let iterator = (*handle).0.begin((*txn).as_txn());
    LmdbIteratorHandle::new2(iterator)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_pending_store_begin_at_key(
    handle: *mut LmdbPendingStoreHandle,
    txn: *mut TransactionHandle,
    key: *const PendingKeyDto,
) -> *mut LmdbIteratorHandle {
    let key = PendingKey::from(&*key);
    let iterator = (*handle).0.begin_at_key((*txn).as_txn(), &key);
    LmdbIteratorHandle::new2(iterator)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_pending_store_exists(
    handle: *mut LmdbPendingStoreHandle,
    txn: *mut TransactionHandle,
    key: *const PendingKeyDto,
) -> bool {
    (*handle)
        .0
        .exists((*txn).as_txn(), &PendingKey::from(&*key))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_pending_store_any(
    handle: *mut LmdbPendingStoreHandle,
    txn: *mut TransactionHandle,
    account: *const u8,
) -> bool {
    (*handle)
        .0
        .any((*txn).as_txn(), &Account::from_ptr(account))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_lmdb_pending_store_for_each_par(
    handle: *mut LmdbPendingStoreHandle,
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
