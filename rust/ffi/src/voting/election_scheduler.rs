use std::ffi::c_void;

use rsnano_core::Account;
use rsnano_node::voting::{ElectionScheduler, ELECTION_SCHEDULER_ACTIVATE_INTERNAL_CALLBACK};
use rsnano_store_lmdb::LmdbReadTransaction;
use rsnano_store_traits::Transaction;

use crate::ledger::datastore::{TransactionHandle, TransactionType};

pub struct ElectionSchedulerHandle(ElectionScheduler);

pub type ElectionSchedulerActivateCallback =
    unsafe extern "C" fn(*mut c_void, *const u8, *mut TransactionHandle);
pub static mut ELECTION_SCHEDULER_ACTIVATE_CALLBACK: Option<ElectionSchedulerActivateCallback> =
    None;

#[no_mangle]
pub extern "C" fn rsn_election_scheduler_create(
    cpp_election_scheduler: *mut c_void,
) -> *mut ElectionSchedulerHandle {
    Box::into_raw(Box::new(ElectionSchedulerHandle(ElectionScheduler::new(
        cpp_election_scheduler,
    ))))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_scheduler_destroy(handle: *mut ElectionSchedulerHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_election_scheduler_activate(
    handle: *mut ElectionSchedulerHandle,
    account: *const u8,
    txn: *const TransactionHandle,
) {
    (*handle)
        .0
        .activate(&Account::from_ptr(account), (*txn).as_txn());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_election_scheduler_activate(
    f: ElectionSchedulerActivateCallback,
) {
    ELECTION_SCHEDULER_ACTIVATE_INTERNAL_CALLBACK = Some(forward_scheduler_activate);
    ELECTION_SCHEDULER_ACTIVATE_CALLBACK = Some(f);
}

fn forward_scheduler_activate(
    cpp_scheduler: *mut c_void,
    account: &Account,
    txn: &dyn Transaction,
) {
    let callback = unsafe {
        ELECTION_SCHEDULER_ACTIVATE_CALLBACK.expect("ELECTION_SCHEDULER_ACTIVATE_CALLBACK missing")
    };

    let txn_handle = TransactionHandle::new(TransactionType::ReadRef(unsafe {
        std::mem::transmute::<&LmdbReadTransaction, &'static LmdbReadTransaction>(
            txn.as_any().downcast_ref::<LmdbReadTransaction>().unwrap(),
        )
    }));
    unsafe {
        callback(cpp_scheduler, account.as_bytes().as_ptr(), txn_handle);
    }
    drop(unsafe { Box::from_raw(txn_handle) });
}
