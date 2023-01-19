use std::ffi::c_void;

use rsnano_core::Account;
use rsnano_store_traits::Transaction;

pub type ElectionSchedulerActivateInternalCallback = fn(*mut c_void, &Account, &dyn Transaction);
pub static mut ELECTION_SCHEDULER_ACTIVATE_INTERNAL_CALLBACK: Option<
    ElectionSchedulerActivateInternalCallback,
> = None;

pub struct ElectionScheduler {
    cpp_election_scheduler: *mut c_void,
}

impl ElectionScheduler {
    pub fn new(cpp_election_scheduler: *mut c_void) -> Self {
        Self {
            cpp_election_scheduler,
        }
    }

    pub fn activate(&self, account: &Account, txn: &dyn Transaction) {
        let callback = unsafe {
            ELECTION_SCHEDULER_ACTIVATE_INTERNAL_CALLBACK
                .expect("ELECTION_SCHEDULER_ACTIVATE_INTERNAL_CALLBACK not defined")
        };
        callback(self.cpp_election_scheduler, account, txn);
    }
}
