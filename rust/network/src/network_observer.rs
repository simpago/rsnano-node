use crate::{ChannelDirection, ChannelInfo, NetworkError};
use std::net::SocketAddrV6;

pub trait NetworkObserver: Send + Sync {
    fn send_succeeded(&self, _buf_size: usize) {}
    fn send_failed(&self) {}
    fn read_succeeded(&self, _count: usize) {}
    fn read_failed(&self) {}
    fn channel_timed_out(&self, _channel: &ChannelInfo) {}
    fn connection_attempt(&self, _peer: &SocketAddrV6) {}
    fn accepted(&self, _peer: &SocketAddrV6, _direction: ChannelDirection) {}
    fn error(&self, _error: NetworkError, _peer: &SocketAddrV6, _direction: ChannelDirection) {}
}

pub struct NullNetworkObserver {}

impl NullNetworkObserver {
    pub fn new() -> Self {
        Self {}
    }
}

impl NetworkObserver for NullNetworkObserver {}
