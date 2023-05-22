use rsnano_core::BlockHash;
use rsnano_node::{bootstrap::BulkPullServer, messages::BulkPull};

use crate::{
    copy_hash_bytes,
    messages::{downcast_message, MessageHandle},
};

pub struct BulkPullServerHandle(BulkPullServer);

#[no_mangle]
pub unsafe extern "C" fn rsn_bulk_pull_server_create(
    request: *mut MessageHandle,
) -> *mut BulkPullServerHandle {
    let msg = downcast_message::<BulkPull>(request);
    Box::into_raw(Box::new(BulkPullServerHandle(BulkPullServer::new(
        msg.clone(),
    ))))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bulk_pull_server_destroy(handle: *mut BulkPullServerHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bulk_pull_server_sent_count(
    handle: *const BulkPullServerHandle,
) -> u32 {
    (*handle).0.sent_count
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bulk_pull_server_sent_count_set(
    handle: *mut BulkPullServerHandle,
    value: u32,
) {
    (*handle).0.sent_count = value;
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bulk_pull_server_max_count(
    handle: *const BulkPullServerHandle,
) -> u32 {
    (*handle).0.max_count
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bulk_pull_server_max_count_set(
    handle: *mut BulkPullServerHandle,
    value: u32,
) {
    (*handle).0.max_count = value;
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bulk_pull_server_include_start(
    handle: *const BulkPullServerHandle,
) -> bool {
    (*handle).0.include_start
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bulk_pull_server_include_start_set(
    handle: *mut BulkPullServerHandle,
    value: bool,
) {
    (*handle).0.include_start = value;
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bulk_pull_server_current(
    handle: *const BulkPullServerHandle,
    result: *mut u8,
) {
    copy_hash_bytes((*handle).0.current, result);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bulk_pull_server_current_set(
    handle: *mut BulkPullServerHandle,
    current: *const u8,
) {
    (*handle).0.current = BlockHash::from_ptr(current);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bulk_pull_server_request(
    handle: *mut BulkPullServerHandle,
) -> *mut MessageHandle {
    MessageHandle::new(Box::new((*handle).0.request.clone()))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bulk_pull_server_request_set_end(
    handle: *mut BulkPullServerHandle,
    end: *const u8,
) {
    (*handle).0.request.end = BlockHash::from_ptr(end);
}
