use std::ffi::c_void;

use crate::{
    ffi::{FfiStream, NetworkConstantsDto},
    messages::{BulkPush, Message},
};

use super::{
    create_message_handle, create_message_handle2, downcast_message, downcast_message_mut,
    MessageHandle, MessageHeaderHandle,
};

#[no_mangle]
pub unsafe extern "C" fn rsn_message_bulk_push_create(
    constants: *mut NetworkConstantsDto,
) -> *mut MessageHandle {
    create_message_handle(constants, BulkPush::new)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_bulk_push_create2(
    header: *mut MessageHeaderHandle,
) -> *mut MessageHandle {
    create_message_handle2(header, BulkPush::with_header)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_bulk_push_deserialize(
    handle: *mut MessageHandle,
    stream: *mut c_void,
) -> bool {
    let mut stream = FfiStream::new(stream);
    downcast_message_mut::<BulkPush>(handle)
        .deserialize(&mut stream)
        .is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_bulk_push_serialize(
    handle: *mut MessageHandle,
    stream: *mut c_void,
) -> bool {
    let mut stream = FfiStream::new(stream);
    downcast_message::<BulkPush>(handle)
        .serialize(&mut stream)
        .is_ok()
}
