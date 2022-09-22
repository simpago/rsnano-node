use num::FromPrimitive;
use std::{ffi::c_void, ops::Deref};

use crate::{
    messages::{MessageHeader, MessageType},
    BlockType, NetworkConstants, Networks,
};

use crate::ffi::{FfiStream, NetworkConstantsDto, StringDto};

#[no_mangle]
pub unsafe extern "C" fn rsn_message_type_to_string(msg_type: u8, result: *mut StringDto) {
    (*result) = match MessageType::from_u8(msg_type) {
        Some(msg_type) => msg_type.as_str().into(),
        None => "n/a".into(),
    }
}

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
pub unsafe extern "C" fn rsn_message_header_create(
    constants: *const NetworkConstantsDto,
    message_type: u8,
    version_using: i16,
) -> *mut MessageHeaderHandle {
    let message_type = MessageType::from_u8(message_type).unwrap();
    let constants = NetworkConstants::try_from(&*constants).unwrap();
    let header = if version_using < 0 {
        MessageHeader::new(&constants, message_type)
    } else {
        MessageHeader::with_version_using(&constants, message_type, version_using as u8)
    };
    Box::into_raw(Box::new(MessageHeaderHandle(header)))
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
pub unsafe extern "C" fn rsn_message_header_version_min(handle: *mut MessageHeaderHandle) -> u8 {
    (*handle).0.version_min()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_header_version_max(handle: *mut MessageHeaderHandle) -> u8 {
    (*handle).0.version_max()
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
pub unsafe extern "C" fn rsn_message_header_extensions(handle: *mut MessageHeaderHandle) -> u16 {
    (*handle).0.extensions()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_header_test_extension(
    handle: *mut MessageHeaderHandle,
    position: usize,
) -> bool {
    (*handle).0.test_extension(position)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_header_set_extension(
    handle: *mut MessageHeaderHandle,
    position: usize,
    value: bool,
) {
    (*handle).0.set_extension(position, value)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_header_set_extensions(
    handle: *mut MessageHeaderHandle,
    value: u16,
) {
    (*handle).0.set_extensions(value)
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
pub unsafe extern "C" fn rsn_message_header_to_string(
    handle: *mut MessageHeaderHandle,
    result: *mut StringDto,
) {
    (*result) = (*handle).0.to_string().into()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_header_block_type(handle: *mut MessageHeaderHandle) -> u8 {
    (*handle).0.block_type() as u8
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_header_set_block_type(
    handle: *mut MessageHeaderHandle,
    block_type: u8,
) {
    (*handle)
        .0
        .set_block_type(BlockType::from_u8(block_type).unwrap_or(BlockType::Invalid));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_header_count(handle: *mut MessageHeaderHandle) -> u8 {
    (*handle).0.count()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_header_set_count(handle: *mut MessageHeaderHandle, count: u8) {
    (*handle).0.set_count(count);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_header_payload_length(
    handle: *mut MessageHeaderHandle,
) -> usize {
    (*handle).0.payload_length()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_header_is_valid_message_type(
    handle: *mut MessageHeaderHandle,
) -> bool {
    (*handle).0.is_valid_message_type()
}
