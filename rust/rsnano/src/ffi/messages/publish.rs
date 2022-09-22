use std::{ffi::c_void, ops::Deref};

use crate::{
    ffi::{BlockHandle, BlockUniquerHandle, FfiStream, NetworkConstantsDto},
    messages::{Message, Publish},
};

use super::{
    create_message_handle, create_message_handle2, downcast_message, downcast_message_mut,
    message_handle_clone, MessageHandle, MessageHeaderHandle,
};

#[no_mangle]
pub unsafe extern "C" fn rsn_message_publish_create(
    constants: *mut NetworkConstantsDto,
    block: *mut BlockHandle,
) -> *mut MessageHandle {
    create_message_handle(constants, |consts| {
        let block = (*block).block.clone();
        Publish::new(consts, block)
    })
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_publish_create2(
    header: *mut MessageHeaderHandle,
    digest: *const u8,
) -> *mut MessageHandle {
    let digest = u128::from_be_bytes(std::slice::from_raw_parts(digest, 16).try_into().unwrap());
    create_message_handle2(header, |consts| Publish::with_header(consts, digest))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_publish_clone(
    handle: *mut MessageHandle,
) -> *mut MessageHandle {
    message_handle_clone::<Publish>(handle)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_publish_serialize(
    handle: *mut MessageHandle,
    stream: *mut c_void,
) -> bool {
    let mut stream = FfiStream::new(stream);
    downcast_message::<Publish>(handle)
        .serialize(&mut stream)
        .is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_publish_deserialize(
    handle: *mut MessageHandle,
    stream: *mut c_void,
    uniquer: *mut BlockUniquerHandle,
) -> bool {
    let mut stream = FfiStream::new(stream);
    let uniquer = if uniquer.is_null() {
        None
    } else {
        Some((*uniquer).deref().as_ref())
    };
    downcast_message_mut::<Publish>(handle)
        .deserialize(&mut stream, uniquer)
        .is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_publish_block(handle: *mut MessageHandle) -> *mut BlockHandle {
    match &downcast_message::<Publish>(handle).block {
        Some(b) => Box::into_raw(Box::new(BlockHandle::new(b.clone()))),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_publish_digest(handle: *mut MessageHandle, result: *mut u8) {
    let result_slice = std::slice::from_raw_parts_mut(result, 16);
    let digest = downcast_message::<Publish>(handle).digest;
    result_slice.copy_from_slice(&digest.to_be_bytes());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_publish_set_digest(
    handle: *mut MessageHandle,
    digest: *const u8,
) {
    let bytes = std::slice::from_raw_parts(digest, 16);
    let digest = u128::from_be_bytes(bytes.try_into().unwrap());
    downcast_message_mut::<Publish>(handle).digest = digest;
}
