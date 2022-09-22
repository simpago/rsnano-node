use std::ffi::c_void;

use crate::{
    ffi::{copy_account_bytes, FfiStream, NetworkConstantsDto},
    messages::{FrontierReq, Message},
    Account,
};

use super::{
    create_message_handle, create_message_handle2, downcast_message, downcast_message_mut,
    MessageHandle, MessageHeaderHandle,
};

#[no_mangle]
pub unsafe extern "C" fn rsn_message_frontier_req_create(
    constants: *mut NetworkConstantsDto,
) -> *mut MessageHandle {
    create_message_handle(constants, FrontierReq::new)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_frontier_req_create2(
    header: *mut MessageHeaderHandle,
) -> *mut MessageHandle {
    create_message_handle2(header, FrontierReq::with_header)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_frontier_req_clone(
    other: *mut MessageHandle,
) -> *mut MessageHandle {
    MessageHandle::from_message(downcast_message::<FrontierReq>(other).clone())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_frontier_req_set_start(
    handle: *mut MessageHandle,
    account: *const u8,
) {
    downcast_message_mut::<FrontierReq>(handle).start = Account::from_ptr(account);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_frontier_req_set_age(handle: *mut MessageHandle, age: u32) {
    downcast_message_mut::<FrontierReq>(handle).age = age;
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_frontier_req_set_count(
    handle: *mut MessageHandle,
    count: u32,
) {
    downcast_message_mut::<FrontierReq>(handle).count = count;
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_frontier_size() -> usize {
    FrontierReq::serialized_size()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_frontier_req_start(
    handle: *mut MessageHandle,
    account: *mut u8,
) {
    let start = downcast_message::<FrontierReq>(handle).start;
    copy_account_bytes(start, account);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_frontier_req_age(handle: *mut MessageHandle) -> u32 {
    downcast_message::<FrontierReq>(handle).age
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_frontier_req_count(handle: *mut MessageHandle) -> u32 {
    downcast_message::<FrontierReq>(handle).count
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_frontier_req_deserialize(
    handle: *mut MessageHandle,
    stream: *mut c_void,
) -> bool {
    let mut stream = FfiStream::new(stream);
    downcast_message_mut::<FrontierReq>(handle)
        .deserialize(&mut stream)
        .is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_frontier_req_serialize(
    handle: *mut MessageHandle,
    stream: *mut c_void,
) -> bool {
    let mut stream = FfiStream::new(stream);
    downcast_message::<FrontierReq>(handle)
        .serialize(&mut stream)
        .is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_frontier_req_is_confirmed_present(
    handle: *mut MessageHandle,
) -> bool {
    downcast_message::<FrontierReq>(handle).is_confirmed_present()
}
