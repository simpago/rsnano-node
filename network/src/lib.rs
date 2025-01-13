pub mod attempt_container;
pub mod bandwidth_limiter;
mod channel_adapter;
mod channel_info;
mod dead_channel_cleanup;
mod network;
mod network_info;
mod network_observer;
mod peer_connector;
pub mod peer_exclusion;
mod response_server_spawner;
mod tcp_listener;
pub mod token_bucket;
pub mod utils;
pub mod write_queue;

use async_trait::async_trait;
pub use channel_adapter::*;
pub use channel_info::*;
pub use dead_channel_cleanup::*;
pub use network::*;
pub use network_info::*;
pub use network_observer::*;
use num_derive::FromPrimitive;
pub use peer_connector::*;
pub use response_server_spawner::*;
use std::fmt::{Debug, Display};
pub use tcp_listener::*;

#[macro_use]
extern crate anyhow;

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

#[derive(PartialEq, Eq, Clone, Copy, FromPrimitive, Debug)]
pub enum ChannelDirection {
    /// Socket was created by accepting an incoming connection
    Inbound,
    /// Socket was created by initiating an outgoing connection
    Outbound,
}

#[derive(FromPrimitive, Copy, Clone, Debug)]
pub enum TrafficType {
    Generic,
    /// Ascending bootstrap (asc_pull_ack, asc_pull_req) traffic
    Bootstrap,
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

/// Policy to affect at which stage a buffer can be dropped
#[derive(PartialEq, Eq, FromPrimitive, Debug, Clone, Copy)]
pub enum DropPolicy {
    /// Can be dropped by bandwidth limiter (default)
    CanDrop,
    /// Should not be dropped by bandwidth limiter,
    /// but it can still be dropped if the write queue is full
    ShouldNotDrop,
}

#[async_trait]
pub trait AsyncBufferReader {
    async fn read(&self, buffer: &mut [u8], count: usize) -> anyhow::Result<()>;
}
