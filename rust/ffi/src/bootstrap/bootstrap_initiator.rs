use super::{
    bootstrap_attempts::BootstrapAttemptsHandle, bootstrap_connections::BootstrapConnectionsHandle,
    pulls_cache::PullsCacheHandle,
};
use crate::{to_rust_string, transport::EndpointDto};
use rsnano_core::{Account, HashOrAccount};
use rsnano_node::bootstrap::{BootstrapInitiator, BootstrapInitiatorExt};
use std::{ffi::c_char, ops::Deref, sync::Arc};

pub struct BootstrapInitiatorHandle(pub Arc<BootstrapInitiator>);

impl Deref for BootstrapInitiatorHandle {
    type Target = Arc<BootstrapInitiator>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_initiator_destroy(handle: *mut BootstrapInitiatorHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_initiator_bootstrap(
    handle: &BootstrapInitiatorHandle,
    force: bool,
    id: *const c_char,
    frontiers_age: u32,
    start_account: *const u8,
) {
    handle.bootstrap(
        force,
        to_rust_string(id),
        frontiers_age,
        Account::from_ptr(start_account),
    );
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_initiator_bootstrap2(
    handle: &BootstrapInitiatorHandle,
    endpoint: &EndpointDto,
    id: *const c_char,
) {
    handle.bootstrap2(endpoint.into(), to_rust_string(id));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_initiator_bootstrap_lazy(
    handle: &BootstrapInitiatorHandle,
    hash_or_account: *const u8,
    force: bool,
    id: *const c_char,
) -> bool {
    handle.bootstrap_lazy(
        HashOrAccount::from_ptr(hash_or_account),
        force,
        to_rust_string(id),
    )
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_initiator_in_progress(
    handle: &BootstrapInitiatorHandle,
) -> bool {
    handle.in_progress()
}

#[no_mangle]
pub extern "C" fn rsn_bootstrap_initiator_has_lazy_attempt(
    handle: &BootstrapInitiatorHandle,
) -> bool {
    handle.current_legacy_attempt().is_some()
}

#[no_mangle]
pub extern "C" fn rsn_bootstrap_initiator_has_legacy_attempt(
    handle: &BootstrapInitiatorHandle,
) -> bool {
    handle.current_legacy_attempt().is_some()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_initiator_stop(handle: &BootstrapInitiatorHandle) {
    handle.stop();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_initiator_attempts(
    handle: &BootstrapInitiatorHandle,
) -> *mut BootstrapAttemptsHandle {
    BootstrapAttemptsHandle::new(Arc::clone(&handle.attempts))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_initiator_connections(
    handle: &BootstrapInitiatorHandle,
) -> *mut BootstrapConnectionsHandle {
    BootstrapConnectionsHandle::new(Arc::clone(&handle.connections))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_initiator_cache(
    handle: &BootstrapInitiatorHandle,
) -> *mut PullsCacheHandle {
    PullsCacheHandle::new(Arc::clone(&handle.cache))
}
