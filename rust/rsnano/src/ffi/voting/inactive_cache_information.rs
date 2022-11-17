use std::time::{Duration, SystemTime, UNIX_EPOCH};
use crate::core::{Account, BlockHash};
use crate::ffi::core::BlockHandle;
use crate::ffi::voting::election_status::ElectionStatusHandle;
use crate::ffi::voting::inactive_cache_status::InactiveCacheStatusHandle;
use crate::Topic::Bootstrap;
use crate::voting::{ElectionStatus, InactiveCacheInformation};

pub struct InactiveCacheInformationHandle(InactiveCacheInformation);

#[no_mangle]
pub unsafe extern "C" fn rsn_inactive_cache_information_create() -> *mut InactiveCacheInformationHandle {
    let info = InactiveCacheInformation::null();
    Box::into_raw(Box::new(InactiveCacheInformationHandle(info)))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_inactive_cache_information_create1(arrival: i64, hash: *const u8, status: *const InactiveCacheStatusHandle, initial_rep_a: *const u8, initial_timestamp_a: u64) -> *mut InactiveCacheInformationHandle {
    let rsn_arrival = UNIX_EPOCH.checked_add(Duration::from_millis(arrival as u64)).unwrap();
    let rsn_hash = BlockHash::from_ptr(hash);
    let rsn_status = (*status).0.clone();
    let rsn_initial_rep_a = Account::from_ptr(initial_rep_a);
    let info = InactiveCacheInformation::new(rsn_arrival, rsn_hash, rsn_status, rsn_initial_rep_a, initial_timestamp_a);
    Box::into_raw(Box::new(InactiveCacheInformationHandle(info)))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_inactive_cache_information_clone(
    handle: *const InactiveCacheInformationHandle,
) -> *mut InactiveCacheInformationHandle {
    Box::into_raw(Box::new(InactiveCacheInformationHandle((*handle).0.clone())))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_inactive_cache_information_destroy(handle: *mut InactiveCacheInformationHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_inactive_cache_information_get_status(
    handle: *const InactiveCacheInformationHandle,
) -> *mut InactiveCacheStatusHandle {
   Box::into_raw(Box::new(InactiveCacheStatusHandle((*handle).0.status.clone())))
}