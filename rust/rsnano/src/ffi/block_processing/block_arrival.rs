use crate::{block_processing::BlockArrival, core::BlockHash};
pub struct BlockArrivalHandle(BlockArrival);

#[no_mangle]
pub extern "C" fn rsn_block_arrival_create() -> *mut BlockArrivalHandle {
    Box::into_raw(Box::new(BlockArrivalHandle(BlockArrival::new())))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_block_arrival_destroy(handle: *mut BlockArrivalHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_block_arrival_add(
    handle: *mut BlockArrivalHandle,
    hash: *const u8,
) -> bool {
    let mut bytes = [0; 32];
    bytes.copy_from_slice(std::slice::from_raw_parts(hash, 32));
    let hash = BlockHash::from_bytes(bytes);
    (*handle).0.add(&hash)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_block_arrival_recent(
    handle: *mut BlockArrivalHandle,
    hash: *const u8,
) -> bool {
    let mut bytes = [0; 32];
    bytes.copy_from_slice(std::slice::from_raw_parts(hash, 32));
    let hash = BlockHash::from_bytes(bytes);
    (*handle).0.recent(&hash)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_block_arrival_size(handle: *mut BlockArrivalHandle) -> usize {
    (*handle).0.size()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_block_arrival_size_of_element(
    handle: *mut BlockArrivalHandle,
) -> usize {
    (*handle).0.size_of_element()
}
