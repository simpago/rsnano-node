use std::{ops::Deref, sync::Arc};

use crate::transport::NetworkFilter;

pub struct NetworkFilterHandle(Arc<NetworkFilter>);

impl NetworkFilterHandle {
    pub fn new(filter: Arc<NetworkFilter>) -> *mut Self {
        Box::into_raw(Box::new(NetworkFilterHandle(filter)))
    }
}

impl Deref for NetworkFilterHandle {
    type Target = Arc<NetworkFilter>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub extern "C" fn rsn_network_filter_create(size: usize) -> *mut NetworkFilterHandle {
    NetworkFilterHandle::new(Arc::new(NetworkFilter::new(size)))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_network_filter_destroy(handle: *mut NetworkFilterHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_network_filter_apply(
    handle: *mut NetworkFilterHandle,
    bytes: *const u8,
    size: usize,
    digest: *mut u8,
) -> bool {
    let (calc_digest, existed) = (*handle).apply(std::slice::from_raw_parts(bytes, size));
    if !digest.is_null() {
        std::slice::from_raw_parts_mut(digest, 16).copy_from_slice(&calc_digest.to_be_bytes());
    }
    existed
}

#[no_mangle]
pub unsafe extern "C" fn rsn_network_filter_clear(
    handle: *mut NetworkFilterHandle,
    digest: *const [u8; 16],
) {
    let digest = u128::from_be_bytes(*digest);
    (*handle).clear(digest);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_network_filter_clear_many(
    handle: *mut NetworkFilterHandle,
    digests: *const [u8; 16],
    count: usize,
) {
    let digests = std::slice::from_raw_parts(digests, count)
        .iter()
        .map(|bytes| u128::from_be_bytes(*bytes));
    (*handle).clear_many(digests);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_network_filter_clear_bytes(
    handle: *mut NetworkFilterHandle,
    bytes: *const u8,
    count: usize,
) {
    let bytes = std::slice::from_raw_parts(bytes, count);
    (*handle).clear_bytes(bytes);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_network_filter_clear_all(handle: *mut NetworkFilterHandle) {
    (*handle).clear_all();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_network_filter_hash(
    handle: *mut NetworkFilterHandle,
    bytes: *const u8,
    count: usize,
    digest: *mut [u8; 16],
) {
    let bytes = std::slice::from_raw_parts(bytes, count);
    let result = (*handle).hash(bytes);
    (*digest) = result.to_be_bytes();
}
