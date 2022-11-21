use crate::ffi::voting::election_status::ElectionStatusHandle;
use crate::voting::RecentlyCementedCache;
use std::sync::Arc;

pub struct RecentlyCementedCacheHandle(Arc<RecentlyCementedCache>);

#[no_mangle]
pub unsafe extern "C" fn rsn_recently_cemented_cache_create1(
    max_size: usize,
) -> *mut RecentlyCementedCacheHandle {
    let info = RecentlyCementedCache::new(max_size);
    Box::into_raw(Box::new(RecentlyCementedCacheHandle(Arc::new(info))))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_recently_cemented_cache_put(
    handle: *const RecentlyCementedCacheHandle,
    election_status: *const ElectionStatusHandle,
) {
    (*handle).0.put((*election_status).0.clone());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_recently_cemented_cache_size(
    handle: *const RecentlyCementedCacheHandle,
) -> usize {
    (*handle).0.size()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_recently_cemented_cache_list(
    handle: *const RecentlyCementedCacheHandle,
    list: *mut RecentlyCementedCachedDto,
) {
    let amounts = (*handle).0.get_cemented();
    let items: Vec<*mut ElectionStatusHandle> = amounts
        .iter()
        .map(|e| Box::into_raw(Box::new(ElectionStatusHandle(e.clone()))))
        .collect();
    let raw_data = Box::into_raw(Box::new(RecentlyCementedCachedRawData(items)));
    (*list).items = (*raw_data).0.as_ptr();
    (*list).count = (*raw_data).0.len();
    (*list).raw_data = raw_data;
}

pub struct RecentlyCementedCachedRawData(Vec<*mut ElectionStatusHandle>);

#[repr(C)]
pub struct RecentlyCementedCachedDto {
    items: *const *mut ElectionStatusHandle,
    count: usize,
    pub raw_data: *mut RecentlyCementedCachedRawData,
}

#[no_mangle]
pub unsafe extern "C" fn rsn_recently_cemented_cache_destroy_dto(
    list: *mut RecentlyCementedCachedDto,
) {
    drop(Box::from_raw((*list).raw_data))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_recently_cemented_cache_get_cemented_type_size() -> usize {
    RecentlyCementedCache::element_size()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_recently_cemented_cache_clone(
    handle: *const RecentlyCementedCacheHandle,
) -> *mut RecentlyCementedCacheHandle {
    Box::into_raw(Box::new(RecentlyCementedCacheHandle((*handle).0.clone())))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_recently_cemented_cache_destroy(
    handle: *mut RecentlyCementedCacheHandle,
) {
    drop(Box::from_raw(handle))
}
