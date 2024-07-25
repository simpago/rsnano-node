use super::{ChannelHandle, EndpointDto};
use crate::messages::MessageHandle;
use rsnano_core::{utils::system_time_from_nanoseconds, PublicKey};
use rsnano_node::transport::{ChannelEnum, ChannelMode, Network};
use std::{
    net::{Ipv6Addr, SocketAddrV6},
    ops::Deref,
    sync::Arc,
};

pub struct TcpChannelsHandle(pub Arc<Network>);

impl Deref for TcpChannelsHandle {
    type Target = Arc<Network>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub extern "C" fn rsn_tcp_channels_port(handle: &TcpChannelsHandle) -> u16 {
    handle.port()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_channels_destroy(handle: *mut TcpChannelsHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub extern "C" fn rsn_tcp_channels_purge(handle: &TcpChannelsHandle, cutoff_ns: u64) {
    let cutoff = system_time_from_nanoseconds(cutoff_ns);
    handle.purge(cutoff);
}

#[no_mangle]
pub extern "C" fn rsn_tcp_channels_channel_count(handle: &mut TcpChannelsHandle) -> usize {
    handle.count_by_mode(ChannelMode::Realtime)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_channels_not_a_peer(
    handle: &mut TcpChannelsHandle,
    endpoint: &EndpointDto,
    allow_local_peers: bool,
) -> bool {
    handle.not_a_peer(&endpoint.into(), allow_local_peers)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_channels_find_channel(
    handle: &mut TcpChannelsHandle,
    endpoint: &EndpointDto,
) -> *mut ChannelHandle {
    match handle.find_channel_by_remote_addr(&endpoint.into()) {
        Some(channel) => ChannelHandle::new(channel),
        None => std::ptr::null_mut(),
    }
}
#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_channels_random_channels(
    handle: &mut TcpChannelsHandle,
    count: usize,
    min_version: u8,
) -> *mut ChannelListHandle {
    let channels = handle.random_channels(count, min_version);
    Box::into_raw(Box::new(ChannelListHandle(channels)))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_channels_find_node_id(
    handle: &mut TcpChannelsHandle,
    node_id: *const u8,
) -> *mut ChannelHandle {
    let node_id = PublicKey::from_ptr(node_id);
    match handle.find_node_id(&node_id) {
        Some(channel) => ChannelHandle::new(channel),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_channels_random_fill(
    handle: &TcpChannelsHandle,
    endpoints: *mut EndpointDto,
) {
    let endpoints = std::slice::from_raw_parts_mut(endpoints, 8);
    let null_endpoint = SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 0, 0, 0);
    let mut tmp = [null_endpoint; 8];
    handle.random_fill(&mut tmp);
    endpoints
        .iter_mut()
        .zip(&tmp)
        .for_each(|(dto, ep)| *dto = ep.into());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_channels_get_next_channel_id(handle: &TcpChannelsHandle) -> usize {
    handle.get_next_channel_id().as_usize()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_channels_len_sqrt(handle: &TcpChannelsHandle) -> f32 {
    handle.len_sqrt()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_tcp_channels_fanout(handle: &TcpChannelsHandle, scale: f32) -> usize {
    handle.fanout(scale)
}

#[no_mangle]
pub extern "C" fn rsn_tcp_channels_random_fanout(
    handle: &TcpChannelsHandle,
    scale: f32,
) -> *mut ChannelListHandle {
    let channels = handle.random_fanout(scale);
    Box::into_raw(Box::new(ChannelListHandle(channels)))
}

#[no_mangle]
pub extern "C" fn rsn_tcp_channels_flood_message(
    handle: &TcpChannelsHandle,
    msg: &MessageHandle,
    scale: f32,
) {
    handle.flood_message(&msg.message, scale)
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
