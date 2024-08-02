use super::{bandwidth_limiter::OutboundBandwidthLimiterHandle, EndpointDto, NetworkFilterHandle};
use crate::{
    messages::MessageHandle,
    utils::{AsyncRuntimeHandle, ContextWrapper},
    NetworkConstantsDto, StatHandle, VoidPointerCallback,
};
use rsnano_core::{
    utils::{system_time_as_nanoseconds, system_time_from_nanoseconds, NULL_ENDPOINT},
    Account,
};
use rsnano_messages::DeserializedMessage;
use rsnano_node::{
    config::NetworkConstants,
    transport::{Channel, ChannelEnum, ChannelFake, ChannelInProc, ChannelTcp},
};
use std::{ffi::c_void, net::SocketAddrV6, ops::Deref, sync::Arc, time::SystemTime};

pub struct ChannelHandle(Arc<ChannelEnum>);

impl ChannelHandle {
    pub fn new(channel: Arc<ChannelEnum>) -> *mut Self {
        Box::into_raw(Box::new(Self(channel)))
    }
}

impl Deref for ChannelHandle {
    type Target = Arc<ChannelEnum>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub unsafe fn as_fake_channel(handle: *mut ChannelHandle) -> &'static ChannelFake {
    match (*handle).0.as_ref() {
        ChannelEnum::Fake(fake) => fake,
        _ => panic!("expected fake channel"),
    }
}

pub unsafe fn as_inproc_channel(handle: *mut ChannelHandle) -> &'static ChannelInProc {
    match (*handle).0.as_ref() {
        ChannelEnum::InProc(inproc) => inproc,
        _ => panic!("expected inproc channel"),
    }
}

pub unsafe fn as_tcp_channel(handle: *mut ChannelHandle) -> &'static Arc<ChannelTcp> {
    match (*handle).0.as_ref() {
        ChannelEnum::Tcp(tcp) => tcp,
        _ => panic!("expected tcp channel"),
    }
}

pub unsafe fn as_channel(handle: *mut ChannelHandle) -> &'static dyn Channel {
    (*handle).0.deref().deref()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_type(handle: *mut ChannelHandle) -> u8 {
    (*handle).0.get_type() as u8
}

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_destroy(handle: *mut ChannelHandle) {
    drop(Box::from_raw(handle));
}

#[no_mangle]
pub extern "C" fn rsn_channel_close(handle: &mut ChannelHandle) {
    handle.close()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_get_last_packet_received(handle: *mut ChannelHandle) -> u64 {
    system_time_as_nanoseconds(as_channel(handle).get_last_packet_received())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_get_last_packet_sent(handle: *mut ChannelHandle) -> u64 {
    system_time_as_nanoseconds(as_channel(handle).get_last_packet_sent())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_set_last_packet_sent(handle: *mut ChannelHandle) {
    as_channel(handle).set_last_packet_sent(SystemTime::now());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_set_last_packet_sent2(handle: *mut ChannelHandle, time: u64) {
    as_channel(handle).set_last_packet_sent(system_time_from_nanoseconds(time));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_get_node_id(
    handle: *mut ChannelHandle,
    result: *mut u8,
) -> bool {
    match as_channel(handle).get_node_id() {
        Some(id) => {
            std::slice::from_raw_parts_mut(result, 32).copy_from_slice(id.as_bytes());
            true
        }
        None => false,
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_set_node_id(handle: *mut ChannelHandle, id: *const u8) {
    as_channel(handle).set_node_id(Account::from_ptr(id));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_id(handle: *mut ChannelHandle) -> usize {
    as_channel(handle).channel_id().as_usize()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_peering_endpoint(
    handle: &ChannelHandle,
    result: *mut EndpointDto,
) {
    (*result) = handle.peering_endpoint().unwrap_or(NULL_ENDPOINT).into()
}

pub type FfiInboundCallback =
    unsafe extern "C" fn(*mut c_void, *mut MessageHandle, *mut ChannelHandle);

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_inproc_create(
    channel_id: usize,
    network_constants: *const NetworkConstantsDto,
    network_filter: *mut NetworkFilterHandle,
    stats: *mut StatHandle,
    limiter: *mut OutboundBandwidthLimiterHandle,
    source_inbound_callback: FfiInboundCallback,
    source_inbound_context: *mut c_void,
    destination_inbound_callback: FfiInboundCallback,
    destination_inbound_context: *mut c_void,
    delete_context: VoidPointerCallback,
    async_rt: &mut AsyncRuntimeHandle,
    source_endpoint: *const EndpointDto,
    destination_endpoint: *const EndpointDto,
    source_node_id: *const u8,
    destination_node_id: *const u8,
) -> *mut ChannelHandle {
    let network_constants = NetworkConstants::try_from(&*network_constants).unwrap();
    let network_filter = (*network_filter).deref().clone();
    let source_context = ContextWrapper::new(source_inbound_context, delete_context);
    let source_inbound = Arc::new(move |msg: DeserializedMessage, channel| {
        let context = source_context.get_context();
        source_inbound_callback(
            context,
            MessageHandle::new(msg),
            ChannelHandle::new(channel),
        );
    });
    let destination_context = ContextWrapper::new(destination_inbound_context, delete_context);
    let destination_inbound = Arc::new(move |msg: DeserializedMessage, channel| {
        let context = destination_context.get_context();
        destination_inbound_callback(
            context,
            MessageHandle::new(msg),
            ChannelHandle::new(channel),
        );
    });
    ChannelHandle::new(Arc::new(ChannelEnum::InProc(ChannelInProc::new(
        channel_id.into(),
        SystemTime::now(),
        network_constants,
        network_filter,
        (*stats).0.clone(),
        (*limiter).0.clone(),
        source_inbound,
        destination_inbound,
        &async_rt.0,
        (&*source_endpoint).into(),
        (&*destination_endpoint).into(),
        Account::from_ptr(source_node_id),
        Account::from_ptr(destination_node_id),
    ))))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_inproc_network_version(handle: *mut ChannelHandle) -> u8 {
    let inproc = as_inproc_channel(handle);
    inproc.network_version()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_fake_network_version(handle: *mut ChannelHandle) -> u8 {
    let inproc = as_fake_channel(handle);
    inproc.network_version()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_inproc_endpoint(
    handle: *mut ChannelHandle,
    result: *mut EndpointDto,
) {
    let inproc = as_inproc_channel(handle);
    (*result) = inproc.local_endpoint.into()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_fake_create(
    channel_id: usize,
    _async_rt: &mut AsyncRuntimeHandle,
    _limiter: *mut OutboundBandwidthLimiterHandle,
    _stats: *mut StatHandle,
    endpoint: *const EndpointDto,
    network_constants: &NetworkConstantsDto,
) -> *mut ChannelHandle {
    ChannelHandle::new(Arc::new(ChannelEnum::Fake(ChannelFake::new(
        SystemTime::now(),
        channel_id.into(),
        SocketAddrV6::from(&(*endpoint)),
        NetworkConstants::try_from(network_constants)
            .unwrap()
            .protocol_info(),
    ))))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_fake_endpoint(
    handle: *mut ChannelHandle,
    result: *mut EndpointDto,
) {
    *result = as_fake_channel(handle).remote_addr().into();
}
