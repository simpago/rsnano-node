use rsnano_node::transport::{ChannelMode, Network};
use std::{ops::Deref, sync::Arc};

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
pub extern "C" fn rsn_tcp_channels_channel_count(handle: &mut TcpChannelsHandle) -> usize {
    handle
        .info
        .read()
        .unwrap()
        .count_by_mode(ChannelMode::Realtime)
}
