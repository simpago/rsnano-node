use std::sync::Arc;

use crate::{
    bootstrap::ChannelTcpWrapper,
    ffi::transport::{as_tcp_channel, ChannelHandle, SocketHandle},
};

use super::bootstrap_server::TcpServerHandle;

pub struct ChannelTcpWrapperHandle(ChannelTcpWrapper);

impl ChannelTcpWrapperHandle {
    pub fn new(wrapper: ChannelTcpWrapper) -> *mut ChannelTcpWrapperHandle {
        Box::into_raw(Box::new(ChannelTcpWrapperHandle(wrapper)))
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_tcp_wrapper_create(
    channel: *mut ChannelHandle,
    socket: *mut SocketHandle,
    response_server: *mut TcpServerHandle,
) -> *mut ChannelTcpWrapperHandle {
    let channel = as_tcp_channel(channel);
    let response_server = if response_server.is_null() {
        None
    } else {
        Some(Arc::clone(&*response_server))
    };

    ChannelTcpWrapperHandle::new(ChannelTcpWrapper::new(
        Arc::clone(channel),
        Arc::clone(&*socket),
        response_server,
    ))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_tcp_wrapper_destroy(handle: *mut ChannelTcpWrapperHandle) {
    drop(Box::from_raw(handle))
}
