use crate::ffi::voting::election_status::ElectionStatusHandle;
use crate::ffi::{copy_account_bytes, copy_amount_bytes, copy_u128_bytes, into_16_byte_array};
use crate::voting::{ElectionStatus, InactiveCacheStatus};

pub struct InactiveCacheStatusHandle(pub(crate) InactiveCacheStatus);

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

#[no_mangle]
pub unsafe extern "C" fn rsn_inactive_cache_status_set_bootstrap_started(
    handle: *mut InactiveCacheStatusHandle,
    bootstrap_started: bool,
) {
    (*handle).0.bootstrap_started = bootstrap_started;
}

#[no_mangle]
pub unsafe extern "C" fn rsn_inactive_cache_status_set_election_started(
    handle: *mut InactiveCacheStatusHandle,
    election_started: bool,
) {
    (*handle).0.election_started = election_started;
}

#[no_mangle]
pub unsafe extern "C" fn rsn_inactive_cache_status_set_confirmed(
    handle: *mut InactiveCacheStatusHandle,
    confirmed: bool,
) {
    (*handle).0.confirmed = confirmed;
}

#[no_mangle]
pub unsafe extern "C" fn rsn_inactive_cache_status_set_tally(
    handle: *mut InactiveCacheStatusHandle,
    tally: *const u8,
) {
    (*handle).0.tally = u128::from_be_bytes(into_16_byte_array(tally));
}
