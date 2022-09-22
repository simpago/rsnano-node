use std::ffi::c_void;

use crate::ffi::bootstrap::BOOTSTRAP_INITIATOR_CLEAR_PULLS_CALLBACK;

pub struct BootstrapInitiator {
    handle: *mut c_void,
}

impl BootstrapInitiator {
    pub(crate) fn new(handle: *mut c_void) -> Self {
        Self { handle }
    }

    pub(crate) fn clear_pulls(&self, bootstrap_id: u64) {
        unsafe {
            match BOOTSTRAP_INITIATOR_CLEAR_PULLS_CALLBACK {
                Some(f) => f(self.handle, bootstrap_id),
                None => panic!("BOOTSTRAP_INITIATOR_CLEAR_PULLS_CALLBACK missing"),
            }
        }
    }
}
