use crate::ffi::{copy_amount_bytes, copy_u128_bytes};
use crate::ffi::voting::election_status::ElectionStatusHandle;
use crate::voting::{ElectionStatus, InactiveCacheStatus};

pub struct InactiveCacheStatusHandle(InactiveCacheStatus);

#[no_mangle]
pub unsafe extern "C" fn rsn_inactive_cache_status_create() -> *mut InactiveCacheStatusHandle {
    let info = InactiveCacheStatus::default();
    Box::into_raw(Box::new(InactiveCacheStatusHandle(info)))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_inactive_cache_status_bootstrap_started(
    handle: *const InactiveCacheStatusHandle,
) -> bool {
    (*handle).0.bootstrap_started
}

#[no_mangle]
pub unsafe extern "C" fn rsn_inactive_cache_status_election_started(
    handle: *const InactiveCacheStatusHandle,
) -> bool {
    (*handle).0.election_started
}

#[no_mangle]
pub unsafe extern "C" fn rsn_inactive_cache_status_confirmed(
    handle: *const InactiveCacheStatusHandle,
) -> bool {
    (*handle).0.confirmed
}

#[no_mangle]
pub unsafe extern "C" fn rsn_inactive_cache_status_tally(
    handle: *const InactiveCacheStatusHandle,
    result: *mut u8,
) {
    copy_u128_bytes((*handle).0.tally, result);
}