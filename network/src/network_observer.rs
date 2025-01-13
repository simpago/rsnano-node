use anyhow::Error;

use crate::{Channel, ChannelDirection, NetworkError, TrafficType};
use std::net::SocketAddrV6;

pub trait NetworkObserver: Send + Sync {
    fn send_succeeded(&self, _buf_size: usize, _traffic_type: TrafficType) {}
    fn send_failed(&self) {}
    fn read_succeeded(&self, _count: usize) {}
    fn read_failed(&self) {}
    fn channel_timed_out(&self, _channel: &Channel) {}
    fn connection_attempt(&self, _peer: &SocketAddrV6) {}
    fn accepted(&self, _peer: &SocketAddrV6, _direction: ChannelDirection) {}
    fn error(&self, _error: NetworkError, _peer: &SocketAddrV6, _direction: ChannelDirection) {}
    fn connect_error(&self, _peer: SocketAddrV6, _e: Error) {}
    fn attempt_timeout(&self, _peer: SocketAddrV6) {}
    fn attempt_cancelled(&self, _peer: SocketAddrV6) {}
    fn merge_peer(&self) {}
    fn accept_failure(&self) {}
}

pub struct NullNetworkObserver {}

impl NullNetworkObserver {
    pub fn new() -> Self {
        Self {}
    }
}

impl NetworkObserver for NullNetworkObserver {}
