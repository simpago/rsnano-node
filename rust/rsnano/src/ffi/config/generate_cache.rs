use std::ops::Deref;

use crate::GenerateCache;

pub struct GenerateCacheHandle(GenerateCache);

impl GenerateCacheHandle {
    pub fn new(cfg: GenerateCache) -> *mut GenerateCacheHandle {
        Box::into_raw(Box::new(GenerateCacheHandle(cfg)))
    }
}

impl Deref for GenerateCacheHandle {
    type Target = GenerateCache;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub extern "C" fn rsn_generate_cache_create() -> *mut GenerateCacheHandle {
    GenerateCacheHandle::new(GenerateCache::new())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_generate_cache_destroy(handle: *mut GenerateCacheHandle) {
    drop(Box::from_raw(handle));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_generate_cache_clone(
    handle: *mut GenerateCacheHandle,
) -> *mut GenerateCacheHandle {
    GenerateCacheHandle::new((*handle).0.clone())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_generate_cache_enable_all(handle: *mut GenerateCacheHandle) {
    (*handle).0.enable_all();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_generate_cache_reps(handle: *mut GenerateCacheHandle) -> bool {
    (*handle).0.reps
}

#[no_mangle]
pub unsafe extern "C" fn rsn_generate_cache_set_reps(
    handle: *mut GenerateCacheHandle,
    enable: bool,
) {
    (*handle).0.reps = enable;
}

#[no_mangle]
pub unsafe extern "C" fn rsn_generate_cache_cemented_count(
    handle: *mut GenerateCacheHandle,
) -> bool {
    (*handle).0.cemented_count
}

#[no_mangle]
pub unsafe extern "C" fn rsn_generate_cache_set_cemented_count(
    handle: *mut GenerateCacheHandle,
    enable: bool,
) {
    (*handle).0.cemented_count = enable;
}

#[no_mangle]
pub unsafe extern "C" fn rsn_generate_cache_unchecked_count(
    handle: *mut GenerateCacheHandle,
) -> bool {
    (*handle).0.unchecked_count
}

#[no_mangle]
pub unsafe extern "C" fn rsn_generate_cache_set_unchecked_count(
    handle: *mut GenerateCacheHandle,
    enable: bool,
) {
    (*handle).0.unchecked_count = enable;
}

#[no_mangle]
pub unsafe extern "C" fn rsn_generate_cache_account_count(
    handle: *mut GenerateCacheHandle,
) -> bool {
    (*handle).0.account_count
}

#[no_mangle]
pub unsafe extern "C" fn rsn_generate_cache_set_account_count(
    handle: *mut GenerateCacheHandle,
    enable: bool,
) {
    (*handle).0.account_count = enable;
}

#[no_mangle]
pub unsafe extern "C" fn rsn_generate_cache_block_count(handle: *mut GenerateCacheHandle) -> bool {
    (*handle).0.block_count
}
