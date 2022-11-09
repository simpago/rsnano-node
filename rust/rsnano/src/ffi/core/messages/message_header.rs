use num::FromPrimitive;
use std::{ffi::c_void, ops::Deref};

use crate::config::NetworkConstants;
use crate::core::messages::{MessageHeader, MessageType};
use crate::core::Networks;

use crate::ffi::utils::FfiStream;

pub struct MessageHeaderHandle(MessageHeader);

impl MessageHeaderHandle {
    pub fn new(header: MessageHeader) -> Self {
        Self(header)
    }
}

impl Deref for MessageHeaderHandle {
    type Target = MessageHeader;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_header_empty() -> *mut MessageHeaderHandle {
    let message_type = MessageType::Invalid;
    let constants = NetworkConstants::empty();
    let header = MessageHeader::new(&constants, message_type);
    Box::into_raw(Box::new(MessageHeaderHandle(header)))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_header_clone(
    handle: *mut MessageHeaderHandle,
) -> *mut MessageHeaderHandle {
    Box::into_raw(Box::new(MessageHeaderHandle((*handle).0.clone())))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_header_destroy(handle: *mut MessageHeaderHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_header_version_using(handle: *mut MessageHeaderHandle) -> u8 {
    (*handle).0.version_using()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_header_set_version_using(
    handle: *mut MessageHeaderHandle,
    version: u8,
) {
    (*handle).0.set_version_using(version);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_header_network(handle: *mut MessageHeaderHandle) -> u16 {
    (*handle).0.network() as u16
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_header_set_network(
    handle: *mut MessageHeaderHandle,
    network: u16,
) {
    (*handle)
        .0
        .set_network(Networks::from_u16(network).unwrap())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_header_size() -> usize {
    MessageHeader::serialized_size()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_header_type(handle: *mut MessageHeaderHandle) -> u8 {
    (*handle).0.message_type() as u8
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_header_deserialize(
    handle: *mut MessageHeaderHandle,
    stream: *mut c_void,
) -> bool {
    let mut stream = FfiStream::new(stream);
    (*handle).0.deserialize(&mut stream).is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_header_serialize(
    handle: *mut MessageHeaderHandle,
    stream: *mut c_void,
) -> bool {
    let mut stream = FfiStream::new(stream);
    (*handle).0.serialize(&mut stream).is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_header_block_type(handle: *mut MessageHeaderHandle) -> u8 {
    (*handle).0.block_type() as u8
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_header_set_extension(
    handle: *mut MessageHeaderHandle,
    position: usize,
    value: bool,
) {
    (*handle).0.set_extension(position, value)
}
