pub mod attempt_container;
pub mod bandwidth_limiter;
mod channel;
mod dead_channel_cleanup;
mod network;
mod network_observer;
mod peer_connector;
pub mod peer_exclusion;
mod tcp_channel_adapter;
mod tcp_listener;
mod tcp_network_adapter;
pub mod token_bucket;
pub mod utils;
pub mod write_queue;

use async_trait::async_trait;
pub use channel::*;
pub use dead_channel_cleanup::*;
pub use network::*;
pub use network_observer::*;
use num_derive::FromPrimitive;
pub use peer_connector::*;
use std::{
    fmt::{Debug, Display},
    sync::Arc,
};
pub use tcp_channel_adapter::*;
pub use tcp_listener::*;
pub use tcp_network_adapter::*;

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

#[derive(FromPrimitive, Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TrafficType {
    Generic,
    /// Ascending bootstrap (asc_pull_ack, asc_pull_req) traffic
    BootstrapServer,
    BootstrapRequests,
    BlockBroadcast,
    BlockBroadcastInitial,
    BlockBroadcastRpc,
    ConfirmationRequests,
    Keepalive,
    Vote,
    VoteRebroadcast,
    RepCrawler,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, FromPrimitive)]
pub enum ChannelMode {
    /// No messages have been exchanged yet, so the mode is undefined
    Undefined,
    /// serve realtime traffic (votes, new blocks,...)
    Realtime,
}

impl ChannelMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChannelMode::Undefined => "undefined",
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

pub trait DataReceiverFactory {
    fn create_receiver_for(&self, channel: Arc<Channel>) -> Box<dyn DataReceiver + Send>;
}

pub enum ReceiveResult {
    Continue,
    Abort,
    Pause,
}

pub trait DataReceiver {
    fn receive(&mut self, data: &[u8]) -> ReceiveResult;
    /// after receive returns Pause this has to be called until it returns true
    fn try_unpause(&self) -> ReceiveResult;
}

pub struct NullDataReceiverFactory;

impl NullDataReceiverFactory {
    pub fn new() -> Self {
        Self
    }
}

impl DataReceiverFactory for NullDataReceiverFactory {
    fn create_receiver_for(&self, _channel: Arc<Channel>) -> Box<dyn DataReceiver + Send> {
        Box::new(NullDataReceiver::new())
    }
}

pub struct NullDataReceiver;

impl NullDataReceiver {
    pub fn new() -> Self {
        Self
    }
}

impl DataReceiver for NullDataReceiver {
    fn receive(&mut self, _: &[u8]) -> ReceiveResult {
        ReceiveResult::Continue
    }

    fn try_unpause(&self) -> ReceiveResult {
        ReceiveResult::Continue
    }
}
