use crate::{
    messages::MessageHandle, transport::ChannelHandle, utils::ContextWrapper, VoidPointerCallback,
};
use rsnano_messages::{DeserializedMessage, Message};
use rsnano_node::bootstrap::{BootstrapServer, BootstrapServerConfig};
use std::{ffi::c_void, ops::Deref, sync::Arc};

pub struct BootstrapServerHandle(pub Arc<BootstrapServer>);

impl Deref for BootstrapServerHandle {
    type Target = Arc<BootstrapServer>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_server_destroy(handle: *mut BootstrapServerHandle) {
    drop(Box::from_raw(handle))
}

pub type BootstrapServerResponseCallback =
    unsafe extern "C" fn(*mut c_void, *mut MessageHandle, *mut ChannelHandle);

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_server_set_callback(
    handle: &BootstrapServerHandle,
    callback: BootstrapServerResponseCallback,
    context: *mut c_void,
    delete_context: VoidPointerCallback,
) {
    let context_wrapper = ContextWrapper::new(context, delete_context);
    handle
        .0
        .set_response_callback(Box::new(move |msg, channel| {
            let msg_handle = MessageHandle::new(DeserializedMessage::new(
                Message::AscPullAck(msg.clone()),
                Default::default(),
            ));

            let channel_handle = ChannelHandle::new(Arc::clone(channel));
            callback(context_wrapper.get_context(), msg_handle, channel_handle);
        }));
}

#[repr(C)]
pub struct BootstrapServerConfigDto {
    pub max_queue: usize,
    pub threads: usize,
    pub batch_size: usize,
}

impl From<&BootstrapServerConfig> for BootstrapServerConfigDto {
    fn from(value: &BootstrapServerConfig) -> Self {
        Self {
            max_queue: value.max_queue,
            threads: value.threads,
            batch_size: value.batch_size,
        }
    }
}

impl From<&BootstrapServerConfigDto> for BootstrapServerConfig {
    fn from(value: &BootstrapServerConfigDto) -> Self {
        Self {
            max_queue: value.max_queue,
            threads: value.threads,
            batch_size: value.batch_size,
        }
    }
}
