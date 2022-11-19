use crate::ffi::core::BlockHandle;
use crate::voting::{ElectionStatus, Prioritization, ValueType};
use std::ptr;
use num_format::Locale::ha;
use crate::ffi::voting::election_status::ElectionStatusHandle;

pub struct ValueTypeHandle(ValueType);

#[no_mangle]
pub unsafe extern "C" fn rsn_prioritization_create_value_type(time: u64, block: *const BlockHandle) -> *mut ValueTypeHandle {
    let block = (*block).block.clone();
    let info = ValueType::new(time, Some(block));
    Box::into_raw(Box::new(ValueTypeHandle(info)))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_prioritization_get_value_type_time(
    handle: *const ValueTypeHandle,
) -> u64 {
    (*handle).0.get_time()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_prioritization_get_value_type_block(
    handle: *const ValueTypeHandle,
) -> *mut BlockHandle {
    match (*handle).0.get_block() {
        Some(winner) => Box::into_raw(Box::new(BlockHandle::new(winner))),
        None => ptr::null_mut(),
    }
}

pub struct PrioritizationHandle(Prioritization);

#[no_mangle]
pub unsafe extern "C" fn rsn_prioritization_create(maximum: u64) -> *mut PrioritizationHandle {
    let info = Prioritization::new(maximum);
    Box::into_raw(Box::new(PrioritizationHandle(info)))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_prioritization_size(handle: *const PrioritizationHandle) -> usize {
    (*handle).0.size()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_prioritization_next(handle: *mut PrioritizationHandle) {
    (*handle).0.next()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_prioritization_bucket_count(handle: *mut PrioritizationHandle) -> usize {
    (*handle).0.bucket_count()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_prioritization_bucket_size(handle: *mut PrioritizationHandle, index: usize) -> usize {
    (*handle).0.bucket_size(index)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_prioritization_empty(handle: *mut PrioritizationHandle) -> bool {
    (*handle).0.empty()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_prioritization_seek(handle: *mut PrioritizationHandle) {
    (*handle).0.seek()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_prioritization_pop(handle: *mut PrioritizationHandle) {
    (*handle).0.pop()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_prioritization_push(handle: *mut PrioritizationHandle, time: u64, block: *const BlockHandle) {
    (*handle).0.push(time, (*block).block.clone())
}