use std::{
    ffi::{c_char, c_void, CStr},
    net::{Ipv6Addr, SocketAddrV6},
    ops::Deref,
    sync::{atomic::Ordering, Arc},
};

use rsnano_core::{utils::system_time_from_nanoseconds, KeyPair, PublicKey};
use rsnano_node::{
    config::NodeConfig,
    messages::DeserializedMessage,
    transport::{ChannelEnum, TcpChannels, TcpChannelsExtension, TcpChannelsOptions},
    NetworkParams,
};

use crate::{
    bootstrap::{FfiBootstrapServerObserver, RequestResponseVisitorFactoryHandle},
    messages::MessageHandle,
    utils::{
        AsyncRuntimeHandle, ContainerInfoComponentHandle, ContextWrapper, LoggerHandle, LoggerMT,
        ThreadPoolHandle,
    },
    NetworkParamsDto, NodeConfigDto, NodeFlagsHandle, StatHandle, VoidPointerCallback,
};

use super::{
    peer_exclusion::PeerExclusionHandle, ChannelHandle, EndpointDto, NetworkFilterHandle,
    OutboundBandwidthLimiterHandle, SocketFfiObserver, SynCookiesHandle, TcpMessageManagerHandle,
};

pub struct TcpChannelsHandle(Arc<TcpChannels>);

pub type SinkCallback = unsafe extern "C" fn(*mut c_void, *mut MessageHandle, *mut ChannelHandle);

#[repr(C)]
pub struct TcpChannelsOptionsDto {
    pub node_config: *const NodeConfigDto,
    pub logger: *mut LoggerHandle,
    pub publish_filter: *mut NetworkFilterHandle,
    pub async_rt: *mut AsyncRuntimeHandle,
    pub network: *mut NetworkParamsDto,
    pub stats: *mut StatHandle,
    pub tcp_message_manager: *mut TcpMessageManagerHandle,
    pub port: u16,
    pub flags: *mut NodeFlagsHandle,
    pub sink_handle: *mut c_void,
    pub sink_callback: SinkCallback,
    pub delete_sink: VoidPointerCallback,
    pub limiter: *mut OutboundBandwidthLimiterHandle,
    pub node_id_prv: *const u8,
    pub syn_cookies: *mut SynCookiesHandle,
    pub workers: *mut ThreadPoolHandle,
    pub socket_observer: *mut c_void,
}

impl TryFrom<&TcpChannelsOptionsDto> for TcpChannelsOptions {
    type Error = anyhow::Error;

    fn try_from(value: &TcpChannelsOptionsDto) -> Result<Self, Self::Error> {
        unsafe {
            let context_wrapper = ContextWrapper::new(value.sink_handle, value.delete_sink);
            let callback = value.sink_callback;
            let sink = Box::new(move |msg: DeserializedMessage, channel| {
                callback(
                    context_wrapper.get_context(),
                    MessageHandle::new(msg),
                    ChannelHandle::new(channel),
                )
            });
            let observer = Arc::new(SocketFfiObserver::new(value.socket_observer));

            Ok(Self {
                node_config: NodeConfig::try_from(&*value.node_config)?,
                logger: Arc::new(LoggerMT::new(Box::from_raw(value.logger))),
                publish_filter: (*value.publish_filter).0.clone(),
                network: NetworkParams::try_from(&*value.network)?,
                async_rt: Arc::clone(&(*value.async_rt).0),
                stats: (*value.stats).0.clone(),
                tcp_message_manager: (*value.tcp_message_manager).deref().clone(),
                port: value.port,
                flags: (*value.flags).0.lock().unwrap().clone(),
                sink,
                limiter: (*value.limiter).0.clone(),
                node_id: KeyPair::from_priv_key_bytes(std::slice::from_raw_parts(
                    value.node_id_prv,
                    32,
                ))
                .unwrap(),
                syn_cookies: (*value.syn_cookies).0.clone(),
                workers: (*value.workers).0.clone(),
                observer,
            })
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_channels_create(
    options: &TcpChannelsOptionsDto,
) -> *mut TcpChannelsHandle {
    let channels = Arc::new(TcpChannels::new(
        TcpChannelsOptions::try_from(options).unwrap(),
    ));
    channels.observe();
    Box::into_raw(Box::new(TcpChannelsHandle(channels)))
}

#[no_mangle]
pub extern "C" fn rsn_tcp_channels_set_port(handle: &mut TcpChannelsHandle, port: u16) {
    handle.0.port.store(port, Ordering::SeqCst)
}

pub type NewChannelCallback = unsafe extern "C" fn(*mut c_void, *mut ChannelHandle);

#[no_mangle]
pub extern "C" fn rsn_tcp_channels_stop(handle: &mut TcpChannelsHandle) {
    handle.0.stop();
}

#[no_mangle]
pub extern "C" fn rsn_tcp_channels_on_new_channel(
    handle: &mut TcpChannelsHandle,
    callback_handle: *mut c_void,
    call_callback: NewChannelCallback,
    delete_callback: VoidPointerCallback,
) {
    let context_wrapper = ContextWrapper::new(callback_handle, delete_callback);
    let callback = Arc::new(move |channel| {
        let ctx = context_wrapper.get_context();
        unsafe { call_callback(ctx, ChannelHandle::new(channel)) };
    });
    handle.0.on_new_channel(callback)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_channels_destroy(handle: *mut TcpChannelsHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_channels_purge(handle: *mut TcpChannelsHandle, cutoff_ns: u64) {
    let cutoff = system_time_from_nanoseconds(cutoff_ns);
    let mut guard = (*handle).0.tcp_channels.lock().unwrap();
    guard.purge(cutoff)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_channels_erase_channel_by_endpoint(
    handle: &mut TcpChannelsHandle,
    endpoint: &EndpointDto,
) {
    handle
        .0
        .tcp_channels
        .lock()
        .unwrap()
        .channels
        .remove_by_endpoint(&SocketAddrV6::from(endpoint));
}

#[no_mangle]
pub extern "C" fn rsn_tcp_channels_channel_count(handle: &mut TcpChannelsHandle) -> usize {
    handle.0.tcp_channels.lock().unwrap().channels.len()
}

#[no_mangle]
pub extern "C" fn rsn_tcp_channels_bootstrap_peer(
    handle: &mut TcpChannelsHandle,
    result: &mut EndpointDto,
) {
    let peer = handle.0.tcp_channels.lock().unwrap().bootstrap_peer();
    *result = peer.into();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_channels_list_channels(
    handle: &mut TcpChannelsHandle,
    min_version: u8,
    include_temporary_channels: bool,
) -> *mut ChannelListHandle {
    let channels = handle
        .0
        .tcp_channels
        .lock()
        .unwrap()
        .list(min_version, include_temporary_channels);
    Box::into_raw(Box::new(ChannelListHandle(channels)))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_channels_update_channel(
    handle: &mut TcpChannelsHandle,
    endpoint: &EndpointDto,
) {
    handle
        .0
        .tcp_channels
        .lock()
        .unwrap()
        .update(&endpoint.into())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_channels_set_last_packet_sent(
    handle: &mut TcpChannelsHandle,
    endpoint: &EndpointDto,
    time_ns: u64,
) {
    handle
        .0
        .tcp_channels
        .lock()
        .unwrap()
        .set_last_packet_sent(&endpoint.into(), system_time_from_nanoseconds(time_ns));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_channels_not_a_peer(
    handle: &mut TcpChannelsHandle,
    endpoint: &EndpointDto,
    allow_local_peers: bool,
) -> bool {
    handle.0.not_a_peer(&endpoint.into(), allow_local_peers)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_channels_find_channel(
    handle: &mut TcpChannelsHandle,
    endpoint: &EndpointDto,
) -> *mut ChannelHandle {
    match handle.0.find_channel(&endpoint.into()) {
        Some(channel) => ChannelHandle::new(channel),
        None => std::ptr::null_mut(),
    }
}
#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_channels_random_channels(
    handle: &mut TcpChannelsHandle,
    count: usize,
    min_version: u8,
    include_temporary_channels: bool,
) -> *mut ChannelListHandle {
    let channels = handle
        .0
        .random_channels(count, min_version, include_temporary_channels);
    Box::into_raw(Box::new(ChannelListHandle(channels)))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_channels_get_peers(
    handle: &mut TcpChannelsHandle,
) -> *mut EndpointListHandle {
    let peers = handle.0.get_peers();
    Box::into_raw(Box::new(EndpointListHandle(peers)))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_channels_get_first_channel(
    handle: &mut TcpChannelsHandle,
) -> *mut ChannelHandle {
    ChannelHandle::new(handle.0.get_first_channel().unwrap())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_channels_find_node_id(
    handle: &mut TcpChannelsHandle,
    node_id: *const u8,
) -> *mut ChannelHandle {
    let node_id = PublicKey::from_ptr(node_id);
    match handle.0.find_node_id(&node_id) {
        Some(channel) => ChannelHandle::new(channel),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_channels_collect_container_info(
    handle: &TcpChannelsHandle,
    name: *const c_char,
) -> *mut ContainerInfoComponentHandle {
    let container_info = (*handle)
        .0
        .collect_container_info(CStr::from_ptr(name).to_str().unwrap().to_owned());
    Box::into_raw(Box::new(ContainerInfoComponentHandle(container_info)))
}

#[no_mangle]
pub extern "C" fn rsn_tcp_channels_erase_temporary_channel(
    handle: &TcpChannelsHandle,
    endpoint: &EndpointDto,
) {
    handle.0.erase_temporary_channel(&endpoint.into())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_channels_random_fill(
    handle: &TcpChannelsHandle,
    endpoints: *mut EndpointDto,
) {
    let endpoints = std::slice::from_raw_parts_mut(endpoints, 8);
    let null_endpoint = SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 0, 0, 0);
    let mut tmp = [null_endpoint; 8];
    handle.0.random_fill(&mut tmp);
    endpoints
        .iter_mut()
        .zip(&tmp)
        .for_each(|(dto, ep)| *dto = ep.into());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_channels_set_observer(
    handle: &mut TcpChannelsHandle,
    observer: *mut c_void,
) {
    let observer = Arc::new(FfiBootstrapServerObserver::new(observer));
    handle.0.set_observer(observer);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_channels_set_message_visitor(
    handle: &mut TcpChannelsHandle,
    visitor_factory: &RequestResponseVisitorFactoryHandle,
) {
    handle
        .0
        .set_message_visitor_factory(visitor_factory.0.clone())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_channels_get_next_channel_id(handle: &TcpChannelsHandle) -> usize {
    handle.0.get_next_channel_id()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_channels_process_messages(handle: &TcpChannelsHandle) {
    handle.0.process_messages();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_channels_reachout(
    handle: &TcpChannelsHandle,
    endpoint: &EndpointDto,
) -> bool {
    handle.0.reachout(&endpoint.into())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_channels_excluded_peers(
    handle: &TcpChannelsHandle,
) -> *mut PeerExclusionHandle {
    Box::into_raw(Box::new(PeerExclusionHandle(
        handle.0.excluded_peers.clone(),
    )))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_channels_ongoing_keepalive(handle: &TcpChannelsHandle) {
    handle.0.ongoing_keepalive()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_channels_start_tcp(
    handle: &TcpChannelsHandle,
    endpoint: &EndpointDto,
) {
    handle.0.start_tcp(endpoint.into());
}

pub struct EndpointListHandle(Vec<SocketAddrV6>);

#[no_mangle]
pub unsafe extern "C" fn rsn_endpoint_list_len(handle: &EndpointListHandle) -> usize {
    handle.0.len()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_endpoint_list_get(
    handle: &EndpointListHandle,
    index: usize,
    result: &mut EndpointDto,
) {
    *result = handle.0.get(index).unwrap().into();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_endpoint_list_destroy(handle: *mut EndpointListHandle) {
    drop(Box::from_raw(handle))
}

pub struct ChannelListHandle(Vec<Arc<ChannelEnum>>);

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_list_len(handle: *mut ChannelListHandle) -> usize {
    (*handle).0.len()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_list_get(
    handle: *mut ChannelListHandle,
    index: usize,
) -> *mut ChannelHandle {
    ChannelHandle::new((*handle).0[index].clone())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_channel_list_destroy(handle: *mut ChannelListHandle) {
    drop(Box::from_raw(handle))
}