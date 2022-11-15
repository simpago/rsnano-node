use std::any::Any;
use std::collections::VecDeque;
use std::mem::size_of;
use std::ops::Deref;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};
use bitvec::ptr::Mut;
use toml_edit::value;
use crate::ffi::core::BlockHandle;
use crate::ffi::voting::election_status::ElectionStatusHandle;
use crate::voting::{ElectionStatus, ElectionStatusType, RecentlyCementedCache};

pub struct RecentlyCementedCacheHandle(Arc<RecentlyCementedCache>);

#[no_mangle]
pub unsafe extern "C" fn rsn_recently_cemented_cache_create1(max_size: usize) -> *mut RecentlyCementedCacheHandle {
    let info = RecentlyCementedCache::new(max_size);
    Box::into_raw(Box::new(RecentlyCementedCacheHandle(Arc::new(info))))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_recently_cemented_cache_put(handle: *const RecentlyCementedCacheHandle, election_status: *const ElectionStatusHandle) {
    let mut cemented = (*handle).0.cemented.lock().unwrap();
    cemented.push_back((*election_status).0.clone());
	if cemented.len() > (*handle).0.max_size
	{
		cemented.pop_front ();
	}
}

#[no_mangle]
pub unsafe extern "C" fn rsn_recently_cemented_cache_size(handle: *const RecentlyCementedCacheHandle) -> usize {
    (*handle).0.cemented.lock().unwrap().len()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_recently_cemented_cache_list(handle: *const RecentlyCementedCacheHandle, list: *mut RecentlyCementedCachedDto) {
    let amounts = (*handle).0.get_cemented();
    let items: Vec<RecentlyCementedCacheItemDto> = amounts
        .iter()
        .map(|e| RecentlyCementedCacheItemDto {
            winner: Box::into_raw(Box::new(BlockHandle::new(e.winner.as_ref().unwrap().clone()))),
            tally: e.tally.to_be_bytes(),
            final_tally: e.final_tally.to_be_bytes(),
            confirmation_request_count: e.confirmation_request_count,
            block_count: e.block_count,
            voter_count: e.voter_count,
            election_end: e.election_end.unwrap().duration_since(UNIX_EPOCH).expect("ERROR").as_secs() as i64, // deal with None
            election_duration: e.election_duration.as_secs() as i64,
            election_status_type: e.election_status_type as u8,
        })
        .collect();
    let raw_data = Box::new(RecentlyCementedCachedRawData(items));
    (*list).items = raw_data.0.as_ptr();
    (*list).count = raw_data.0.len();
    (*list).raw_data = Box::into_raw(raw_data);
}

#[repr(C)]
pub struct RecentlyCementedCacheItemDto {
    winner: *mut BlockHandle,
    tally: [u8; 16],
    final_tally: [u8; 16],
    confirmation_request_count: u32,
    block_count: u32,
    voter_count: u32,
    election_end: i64,
    election_duration: i64,
    election_status_type: u8,
}

pub struct RecentlyCementedCachedRawData(Vec<RecentlyCementedCacheItemDto>);

#[repr(C)]
pub struct RecentlyCementedCachedDto {
    items: *const RecentlyCementedCacheItemDto,
    count: usize,
    pub raw_data: *mut RecentlyCementedCachedRawData,
}

#[no_mangle]
pub unsafe extern "C" fn rsn_recently_cemented_cache_destroy_dto(list: *mut RecentlyCementedCachedDto) {
    drop(Box::from_raw((*list).raw_data))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_recently_cemented_cache_get_cemented_size(handle: *const RecentlyCementedCacheHandle) -> usize {
    (*handle).0.cemented.lock().unwrap().len()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_recently_cemented_cache_get_cemented_type_size() -> usize {
    size_of::<ElectionStatus>()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_recently_cemented_cache_clone(
    handle: *const RecentlyCementedCacheHandle,
) -> *mut RecentlyCementedCacheHandle {
    Box::into_raw(Box::new(RecentlyCementedCacheHandle((*handle).0.clone())))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_recently_cemented_cache_destroy(handle: *mut RecentlyCementedCacheHandle) {
    drop(Box::from_raw(handle))
}