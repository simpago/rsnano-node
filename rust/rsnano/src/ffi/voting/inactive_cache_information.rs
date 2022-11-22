use crate::core::{Account, BlockHash};
use crate::ffi::core::BlockHandle;
use crate::ffi::voting::election_status::ElectionStatusHandle;
use crate::ffi::voting::inactive_cache_status::InactiveCacheStatusHandle;
use crate::voting::{ElectionStatus, InactiveCacheInformation};
use crate::Topic::Bootstrap;
use num_format::Locale::ha;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use crate::ffi::copy_hash_bytes;

pub struct InactiveCacheInformationHandle(InactiveCacheInformation);

#[no_mangle]
pub unsafe extern "C" fn rsn_inactive_cache_information_create(
) -> *mut InactiveCacheInformationHandle {
    let info = InactiveCacheInformation::null();
    Box::into_raw(Box::new(InactiveCacheInformationHandle(info)))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_inactive_cache_information_create1(
    arrival: i64,
    hash: *const u8,
    status: *const InactiveCacheStatusHandle,
    initial_rep_a: *const u8,
    initial_timestamp_a: u64,
) -> *mut InactiveCacheInformationHandle {
    let rsn_hash = BlockHash::from_ptr(hash);
    let rsn_status = (*status).0.clone();
    let rsn_initial_rep_a = Account::from_ptr(initial_rep_a);
    let info = InactiveCacheInformation::new(
        Instant::now().checked_sub(Duration::from_secs(arrival as u64)).unwrap(),
        rsn_hash,
        rsn_status,
        rsn_initial_rep_a,
        initial_timestamp_a,
    );
    Box::into_raw(Box::new(InactiveCacheInformationHandle(info)))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_inactive_cache_information_clone(
    handle: *const InactiveCacheInformationHandle,
) -> *mut InactiveCacheInformationHandle {
    Box::into_raw(Box::new(InactiveCacheInformationHandle(
        (*handle).0.clone(),
    )))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_inactive_cache_information_destroy(
    handle: *mut InactiveCacheInformationHandle,
) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_inactive_cache_information_get_arrival(
    handle: *const InactiveCacheInformationHandle,
) -> u64 {
    (*handle)
        .0
        .arrival
        .elapsed()
        .as_secs()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_inactive_cache_information_get_hash(
    handle: *const InactiveCacheInformationHandle,
    result: *mut u8,
)  {
    copy_hash_bytes ((*handle).0.hash, result);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_inactive_cache_information_get_status(
    handle: *const InactiveCacheInformationHandle,
) -> *mut InactiveCacheStatusHandle {
    Box::into_raw(Box::new(InactiveCacheStatusHandle(
        (*handle).0.status.clone(),
    )))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_inactive_cache_information_get_voters(
    handle: *const InactiveCacheInformationHandle,
    vector: *mut VotersDto,
) {
    let voters: Vec<(Account, u64)> = (*handle).0.voters.clone();
    let items: Vec<VotersItemDto> = voters
        .iter()
        .map(|(a, t)| VotersItemDto {
            account: *a.as_bytes(),
            timestamp: *t,
        })
        .collect();
    let raw_data = Box::new(VotersRawData(items));
    (*vector).items = raw_data.0.as_ptr();
    (*vector).count = raw_data.0.len();
    (*vector).raw_data = Box::into_raw(raw_data);
}

#[repr(C)]
pub struct VotersItemDto {
    account: [u8; 32],
    timestamp: u64,
}

pub struct VotersRawData(Vec<VotersItemDto>);

#[repr(C)]
pub struct VotersDto {
    items: *const VotersItemDto,
    count: usize,
    pub raw_data: *mut VotersRawData,
}

#[no_mangle]
pub unsafe extern "C" fn rsn_inactive_cache_information_destroy_dto(vector: *mut VotersDto) {
    drop(Box::from_raw((*vector).raw_data))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_inactive_cache_information_to_string(
    handle: *const InactiveCacheInformationHandle
) { println!("{}", (*handle).0); }
