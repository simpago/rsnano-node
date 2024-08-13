mod attempt_container;
mod bandwidth_limiter;
mod block_deserializer;
mod channel;
mod channel_container;
mod dead_channel_cleanup;
mod fair_queue;
mod handshake_process;
mod inbound_message_queue;
mod latest_keepalives;
mod message_deserializer;
mod message_processor;
mod network;
mod network_filter;
mod network_threads;
mod peer_cache_connector;
mod peer_cache_updater;
mod peer_connector;
mod peer_exclusion;
mod realtime_message_handler;
mod response_server;
mod response_server_factory;
mod syn_cookies;
mod tcp_listener;
mod tcp_stream;
mod tcp_stream_factory;
mod token_bucket;
mod tokio_socket_facade;
mod vec_buffer_reader;
mod write_queue;

pub use bandwidth_limiter::{
    BandwidthLimitType, BandwidthLimiter, OutboundBandwidthLimiter, OutboundBandwidthLimiterConfig,
};
pub use block_deserializer::read_block;
pub use channel::*;
pub(crate) use dead_channel_cleanup::*;
pub(crate) use fair_queue::*;
pub(crate) use handshake_process::*;
pub use inbound_message_queue::InboundMessageQueue;
pub use latest_keepalives::*;
pub use message_deserializer::{AsyncBufferReader, MessageDeserializer};
pub use message_processor::*;
pub use network::*;
pub use network_filter::NetworkFilter;
pub(crate) use network_threads::*;
pub use peer_cache_connector::*;
pub use peer_cache_updater::*;
pub use peer_connector::*;
pub(crate) use peer_exclusion::PeerExclusion;
pub use realtime_message_handler::RealtimeMessageHandler;
pub use response_server::*;
pub(crate) use response_server_factory::*;
use std::fmt::{Debug, Display};
pub use syn_cookies::SynCookies;
pub use tcp_listener::*;
pub use tcp_stream::TcpStream;
pub use tcp_stream_factory::TcpStreamFactory;
use token_bucket::TokenBucket;
pub use tokio_socket_facade::*;
pub use vec_buffer_reader::VecBufferReader;

use crate::stats;

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Hash)]
pub struct ChannelId(usize);

impl ChannelId {
    pub const LOOPBACK: Self = Self(0);
    pub const MIN: Self = Self(usize::MIN);
    pub const MAX: Self = Self(usize::MAX);

    pub fn as_usize(&self) -> usize {
        self.0
    }
}

impl Display for ChannelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl Debug for ChannelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, f)
    }
}

impl From<usize> for ChannelId {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

/// Policy to affect at which stage a buffer can be dropped
#[derive(PartialEq, Eq, FromPrimitive, Debug, Clone, Copy)]
pub enum BufferDropPolicy {
    /// Can be dropped by bandwidth limiter (default)
    Limiter,
    /// Should not be dropped by bandwidth limiter
    NoLimiterDrop,
    /// Should not be dropped by bandwidth limiter or socket write queue limiter
    NoSocketDrop,
}

#[derive(PartialEq, Eq, Clone, Copy, FromPrimitive, Debug)]
pub enum ChannelDirection {
    /// Socket was created by accepting an incoming connection
    Inbound,
    /// Socket was created by initiating an outgoing connection
    Outbound,
}

impl From<ChannelDirection> for stats::Direction {
    fn from(value: ChannelDirection) -> Self {
        match value {
            ChannelDirection::Inbound => stats::Direction::In,
            ChannelDirection::Outbound => stats::Direction::Out,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, FromPrimitive)]
pub enum ChannelMode {
    /// No messages have been exchanged yet, so the mode is undefined
    Undefined,
    /// Only serve bootstrap requests
    Bootstrap,
    /// serve realtime traffic (votes, new blocks,...)
    Realtime,
}

impl ChannelMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChannelMode::Undefined => "undefined",
            ChannelMode::Bootstrap => "bootstrap",
            ChannelMode::Realtime => "realtime",
        }
    }
}

#[derive(FromPrimitive, Copy, Clone, Debug)]
pub enum TrafficType {
    Generic,
    /** For bootstrap (asc_pull_ack, asc_pull_req) traffic */
    Bootstrap,
}
