use crate::transport::ChannelHandle;
use rsnano_core::{BlockHash, Root};
use rsnano_node::consensus::{RequestAggregator, RequestAggregatorConfig};
use std::{ops::Deref, sync::Arc};

pub struct RequestAggregatorHandle(pub Arc<RequestAggregator>);

impl Deref for RequestAggregatorHandle {
    type Target = Arc<RequestAggregator>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_request_aggregator_destroy(handle: *mut RequestAggregatorHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub extern "C" fn rsn_request_aggregator_add(
    handle: &RequestAggregatorHandle,
    channel: &ChannelHandle,
    hashes_roots: &HashesRootsVecHandle,
) -> bool {
    handle
        .0
        .request(hashes_roots.0.clone(), Arc::clone(channel))
}

#[no_mangle]
pub extern "C" fn rsn_request_aggregator_len(handle: &RequestAggregatorHandle) -> usize {
    handle.0.len()
}

pub struct HashesRootsVecHandle(Vec<(BlockHash, Root)>);

#[no_mangle]
pub extern "C" fn rsn_hashes_roots_vec_create() -> *mut HashesRootsVecHandle {
    Box::into_raw(Box::new(HashesRootsVecHandle(Vec::new())))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_hashes_roots_vec_destroy(handle: *mut HashesRootsVecHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_hashes_roots_vec_push(
    handle: &mut HashesRootsVecHandle,
    hash: *const u8,
    root: *const u8,
) {
    handle
        .0
        .push((BlockHash::from_ptr(hash), Root::from_ptr(root)))
}

#[repr(C)]
pub struct RequestAggregatorConfigDto {
    pub threads: usize,
    pub max_queue: usize,
    pub batch_size: usize,
}

impl From<&RequestAggregatorConfigDto> for RequestAggregatorConfig {
    fn from(value: &RequestAggregatorConfigDto) -> Self {
        Self {
            threads: value.threads,
            max_queue: value.max_queue,
            batch_size: value.batch_size,
        }
    }
}

impl From<&RequestAggregatorConfig> for RequestAggregatorConfigDto {
    fn from(value: &RequestAggregatorConfig) -> Self {
        Self {
            threads: value.threads,
            max_queue: value.max_queue,
            batch_size: value.batch_size,
        }
    }
}
