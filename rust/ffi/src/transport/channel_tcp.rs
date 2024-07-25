use super::{
    bandwidth_limiter::OutboundBandwidthLimiterHandle,
    channel::{as_tcp_channel, ChannelHandle},
    socket::SocketHandle,
    EndpointDto, TcpChannelsHandle,
};
use crate::{
    messages::MessageHandle, utils::AsyncRuntimeHandle, ErrorCodeDto, NetworkConstantsDto,
    StatHandle, VoidPointerCallback,
};
use num::FromPrimitive;
use rsnano_node::{
    config::NetworkConstants,
    transport::{BufferDropPolicy, Channel, ChannelEnum, ChannelTcp, TrafficType},
    utils::ErrorCode,
};
use std::{ffi::c_void, ops::Deref, sync::Arc, time::SystemTime};

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_tcp_create(
    socket: *mut SocketHandle,
    stats: &StatHandle,
    tcp_channels: &TcpChannelsHandle,
    limiter: *const OutboundBandwidthLimiterHandle,
    async_rt: &AsyncRuntimeHandle,
    channel_id: usize,
    network_constants: &NetworkConstantsDto,
) -> *mut ChannelHandle {
    let limiter = Arc::clone(&*limiter);
    let async_rt = Arc::clone(&async_rt.0);
    let protocol = NetworkConstants::try_from(network_constants)
        .unwrap()
        .protocol_info();

    ChannelHandle::new(Arc::new(ChannelEnum::Tcp(Arc::new(ChannelTcp::new(
        Arc::clone((*socket).deref()),
        SystemTime::now(),
        Arc::clone(stats),
        limiter,
        &async_rt,
        channel_id.into(),
        protocol,
    )))))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_tcp_local_endpoint(
    handle: *mut ChannelHandle,
    endpoint: *mut EndpointDto,
) {
    (*endpoint) = EndpointDto::from(as_tcp_channel(handle).local_endpoint());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_tcp_remote_endpoint(
    handle: *mut ChannelHandle,
    endpoint: *mut EndpointDto,
) {
    (*endpoint) = EndpointDto::from(as_tcp_channel(handle).remote_endpoint())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_tcp_socket_id(handle: *mut ChannelHandle) -> usize {
    as_tcp_channel(handle).socket_id()
}

pub type ChannelTcpSendBufferCallback =
    unsafe extern "C" fn(*mut c_void, *const ErrorCodeDto, usize);

pub struct SendBufferCallbackWrapper {
    callback: ChannelTcpSendBufferCallback,
    /// `std::function<error_code const&, size_t>*`
    context: *mut c_void,
    delete_callback: VoidPointerCallback,
}

impl SendBufferCallbackWrapper {
    pub fn new(
        callback: ChannelTcpSendBufferCallback,
        context: *mut c_void,
        delete_callback: VoidPointerCallback,
    ) -> Self {
        Self {
            callback,
            context,
            delete_callback,
        }
    }

    pub fn call(&self, ec: ErrorCode, size: usize) {
        let ec_dto = ErrorCodeDto::from(&ec);
        unsafe {
            (self.callback)(self.context, &ec_dto, size);
        }
    }
}

impl Drop for SendBufferCallbackWrapper {
    fn drop(&mut self) {
        unsafe {
            (self.delete_callback)(self.context);
        }
    }
}

unsafe impl Send for SendBufferCallbackWrapper {}
unsafe impl Sync for SendBufferCallbackWrapper {}

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_tcp_network_version(handle: *mut ChannelHandle) -> u8 {
    let tcp = as_tcp_channel(handle);
    tcp.network_version()
}

pub type ChannelTcpSendCallback = unsafe extern "C" fn(*mut c_void, *const ErrorCodeDto, usize);

pub struct ChannelTcpSendCallbackWrapper {
    context: *mut c_void,
    callback: ChannelTcpSendCallback,
    delete: VoidPointerCallback,
}

impl ChannelTcpSendCallbackWrapper {
    pub fn new(
        context: *mut c_void,
        callback: ChannelTcpSendCallback,
        delete: VoidPointerCallback,
    ) -> Self {
        Self {
            context,
            callback,
            delete,
        }
    }

    pub fn call(&self, ec: ErrorCode, size: usize) {
        let ec_dto = ErrorCodeDto::from(&ec);
        unsafe {
            (self.callback)(self.context, &ec_dto, size);
        }
    }
}

impl Drop for ChannelTcpSendCallbackWrapper {
    fn drop(&mut self) {
        unsafe {
            (self.delete)(self.context);
        }
    }
}

unsafe impl Send for ChannelTcpSendCallbackWrapper {}
unsafe impl Sync for ChannelTcpSendCallbackWrapper {}

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_tcp_send(
    handle: *mut ChannelHandle,
    msg: &MessageHandle,
    callback: ChannelTcpSendCallback,
    delete_callback: VoidPointerCallback,
    context: *mut c_void,
    policy: u8,
    traffic_type: u8,
) {
    let callback_wrapper = ChannelTcpSendCallbackWrapper::new(context, callback, delete_callback);
    let callback_box = Box::new(move |ec, size| {
        callback_wrapper.call(ec, size);
    });
    as_tcp_channel(handle).send(
        &msg.message,
        Some(callback_box),
        BufferDropPolicy::from_u8(policy).unwrap(),
        TrafficType::from_u8(traffic_type).unwrap(),
    );
}
