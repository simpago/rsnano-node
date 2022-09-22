use std::{ffi::c_void, sync::Arc};

use crate::{
    bootstrap::{MessageDeserializer, MessageDeserializerExt},
    ffi::{
        messages::MessageHandle, network::SocketHandle, voting::VoteUniquerHandle,
        BlockUniquerHandle, ErrorCodeDto, NetworkConstantsDto, NetworkFilterHandle, StringDto,
        VoidPointerCallback,
    },
    messages::Message,
    stats::DetailType,
    utils::ErrorCode,
    NetworkConstants,
};

pub struct MessageDeserializerHandle(Arc<MessageDeserializer>);

#[no_mangle]
pub unsafe extern "C" fn rsn_message_deserializer_create(
    network_constants: *const NetworkConstantsDto,
    network_filter: *mut NetworkFilterHandle,
    block_uniquer: *mut BlockUniquerHandle,
    vote_uniquer: *mut VoteUniquerHandle,
) -> *mut MessageDeserializerHandle {
    let network_constants = NetworkConstants::try_from(&*network_constants).unwrap();
    let network_filter = Arc::clone(&*network_filter);
    let block_uniquer = Arc::clone(&*block_uniquer);
    let vote_uniquer = Arc::clone(&*vote_uniquer);
    Box::into_raw(Box::new(MessageDeserializerHandle(Arc::new(
        MessageDeserializer::new(
            network_constants,
            network_filter,
            block_uniquer,
            vote_uniquer,
        ),
    ))))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_deserializer_destroy(handle: *mut MessageDeserializerHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_deserializer_status(
    handle: *mut MessageDeserializerHandle,
) -> u8 {
    (*handle).0.status() as u8
}

pub type MessageDeserializedCallback =
    unsafe extern "C" fn(*mut c_void, *const ErrorCodeDto, *mut MessageHandle);

#[no_mangle]
pub unsafe extern "C" fn rsn_message_deserializer_read(
    handle: *mut MessageDeserializerHandle,
    socket: *mut SocketHandle,
    callback: MessageDeserializedCallback,
    destroy_callback: VoidPointerCallback,
    context: *mut c_void,
) {
    let socket = Arc::clone(&*socket);
    let callback_wrapper = ReadCallbackWrapper {
        callback,
        context,
        destroy_callback,
    };
    (*handle).0.read(
        socket,
        Box::new(move |ec, msg| {
            callback_wrapper.callback(ec, msg);
        }),
    );
}

struct ReadCallbackWrapper {
    callback: MessageDeserializedCallback,
    destroy_callback: VoidPointerCallback,
    context: *mut c_void,
}

impl ReadCallbackWrapper {
    pub fn callback(&self, ec: ErrorCode, msg: Option<Box<dyn Message>>) {
        let dto = ErrorCodeDto::from(&ec);
        let msg_handle = match msg {
            Some(m) => MessageHandle::new(m),
            None => std::ptr::null_mut(),
        };
        unsafe {
            (self.callback)(self.context, &dto, msg_handle);
        }
    }
}

impl Drop for ReadCallbackWrapper {
    fn drop(&mut self) {
        unsafe {
            (self.destroy_callback)(self.context);
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_deserializer_parse_status_to_stat_detail(
    handle: *mut MessageDeserializerHandle,
) -> u8 {
    let detail = DetailType::from((*handle).0.status());
    detail as u8
}

#[no_mangle]
pub unsafe extern "C" fn rsn_message_deserializer_parse_status_to_string(
    handle: *mut MessageDeserializerHandle,
    result: *mut StringDto,
) {
    let status = (*handle).0.status().as_str();
    *result = StringDto::from(status);
}
