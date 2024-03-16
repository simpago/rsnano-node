use std::ffi::c_void;

use super::PullInfo;

pub type BootstrapInitiatorClearPullsCallback = unsafe extern "C" fn(*mut c_void, u64);
pub static mut BOOTSTRAP_INITIATOR_CLEAR_PULLS_CALLBACK: Option<
    BootstrapInitiatorClearPullsCallback,
> = None;

pub type BootstrapInitiatorInProgressCallback = unsafe extern "C" fn(*mut c_void) -> bool;
pub static mut BOOTSTRAP_INITIATOR_IN_PROGRESS_CALLBACK: Option<
    BootstrapInitiatorInProgressCallback,
> = None;

pub type BootstrapInitiatorRemoveCacheCallback = fn(*mut c_void, &PullInfo);
pub static mut BOOTSTRAP_INITIATOR_REMOVE_CACHE_CALLBACK: Option<
    BootstrapInitiatorRemoveCacheCallback,
> = None;

pub struct BootstrapInitiator {
    handle: *mut c_void,
}

impl BootstrapInitiator {
    pub fn new(handle: *mut c_void) -> Self {
        Self { handle }
    }

    pub fn clear_pulls(&self, bootstrap_id: u64) {
        unsafe {
            BOOTSTRAP_INITIATOR_CLEAR_PULLS_CALLBACK
                .expect("BOOTSTRAP_INITIATOR_CLEAR_PULLS_CALLBACK missing")(
                self.handle,
                bootstrap_id,
            )
        }
    }

    pub fn in_progress(&self) -> bool {
        unsafe {
            BOOTSTRAP_INITIATOR_IN_PROGRESS_CALLBACK
                .expect("BOOTSTRAP_INITIATOR_IN_PROGRESS_CALLBACK missing")(self.handle)
        }
    }

    pub fn remove_from_cache(&self, pull: &PullInfo) {
        unsafe {
            BOOTSTRAP_INITIATOR_REMOVE_CACHE_CALLBACK
                .expect("BOOTSTRAP_INITIATOR_REMOVE_CACHE_CALLBACK missing")(
                self.handle, pull
            )
        }
    }
}

unsafe impl Send for BootstrapInitiator {}
unsafe impl Sync for BootstrapInitiator {}