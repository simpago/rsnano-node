use std::{ffi::c_void, ops::Deref, sync::Arc};

use crate::bootstrap::BootstrapInitiator;

pub struct BootstrapInitiatorHandle(Arc<BootstrapInitiator>);

impl Deref for BootstrapInitiatorHandle {
    type Target = Arc<BootstrapInitiator>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_initiator_create(
    handle: *mut c_void,
) -> *mut BootstrapInitiatorHandle {
    Box::into_raw(Box::new(BootstrapInitiatorHandle(Arc::new(
        BootstrapInitiator::new(handle),
    ))))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_initiator_destroy(handle: *mut BootstrapInitiatorHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_callback_block_bootstrap_initiator_clear_pulls(
    f: BootstrapInitiatorClearPullsCallback,
) {
    BOOTSTRAP_INITIATOR_CLEAR_PULLS_CALLBACK = Some(f);
}

type BootstrapInitiatorClearPullsCallback = unsafe extern "C" fn(*mut c_void, u64);
pub(crate) static mut BOOTSTRAP_INITIATOR_CLEAR_PULLS_CALLBACK: Option<
    BootstrapInitiatorClearPullsCallback,
> = None;
