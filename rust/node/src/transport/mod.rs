mod attempt_container;
mod bandwidth_limiter;
mod block_deserializer;
mod channel_container;
mod channel_fake;
mod channel_inproc;
mod channel_tcp;
mod fair_queue;
mod handshake_process;
mod inbound_message_queue;
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
mod socket;
mod syn_cookies;
mod tcp_listener;
mod tcp_stream;
mod tcp_stream_factory;
mod token_bucket;
mod tokio_socket_facade;
mod write_queue;

use async_trait::async_trait;
pub use bandwidth_limiter::{
    BandwidthLimitType, BandwidthLimiter, OutboundBandwidthLimiter, OutboundBandwidthLimiterConfig,
};
pub use block_deserializer::read_block;
pub use channel_fake::ChannelFake;
pub use channel_inproc::{ChannelInProc, InboundCallback, VecBufferReader};
pub use channel_tcp::*;
pub use fair_queue::*;
pub(crate) use handshake_process::*;
pub use inbound_message_queue::InboundMessageQueue;
pub use message_deserializer::{AsyncBufferReader, MessageDeserializer};
pub use message_processor::*;
pub use network::*;
pub use network_filter::NetworkFilter;
pub use network_threads::*;
pub use peer_cache_connector::*;
pub use peer_cache_updater::*;
pub use peer_connector::*;
pub use peer_exclusion::PeerExclusion;
pub use realtime_message_handler::RealtimeMessageHandler;
pub use response_server::*;
pub(crate) use response_server_factory::*;
use rsnano_core::Account;
use rsnano_messages::Message;
pub use socket::*;
use std::{
    fmt::{Debug, Display},
    net::SocketAddrV6,
    ops::Deref,
    sync::Arc,
    time::{Duration, SystemTime},
};
pub use syn_cookies::SynCookies;
pub use tcp_listener::*;
pub use tcp_stream::TcpStream;
pub use tcp_stream_factory::TcpStreamFactory;
use token_bucket::TokenBucket;
pub use tokio_socket_facade::*;

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Hash)]
pub struct ChannelId(usize);

impl ChannelId {
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

#[repr(u8)]
#[derive(FromPrimitive, PartialEq, Eq)]
pub enum TransportType {
    Undefined = 0,
    Tcp = 1,
    Loopback = 2,
    Fake = 3,
}

#[async_trait]
pub trait Channel: AsyncBufferReader {
    fn channel_id(&self) -> ChannelId;
    fn get_last_bootstrap_attempt(&self) -> SystemTime; //todo switch back to Instant
    fn set_last_bootstrap_attempt(&self, time: SystemTime); //todo switch back to Instant
    fn get_last_packet_received(&self) -> SystemTime; //todo switch back to Instant
    fn set_last_packet_received(&self, instant: SystemTime); //todo switch back to Instant
    fn get_last_packet_sent(&self) -> SystemTime; //todo switch back to Instant
    fn set_last_packet_sent(&self, instant: SystemTime); //todo switch back to Instant
    fn get_node_id(&self) -> Option<Account>;
    fn set_node_id(&self, id: Account);
    fn is_alive(&self) -> bool;
    fn get_type(&self) -> TransportType;
    fn local_addr(&self) -> SocketAddrV6;
    fn remote_addr(&self) -> SocketAddrV6;
    fn peering_endpoint(&self) -> Option<SocketAddrV6>;
    fn network_version(&self) -> u8;
    fn direction(&self) -> ChannelDirection;
    fn mode(&self) -> ChannelMode;
    fn set_mode(&self, mode: ChannelMode);
    fn set_timeout(&self, timeout: Duration);

    fn try_send(&self, message: &Message, drop_policy: BufferDropPolicy, traffic_type: TrafficType);

    async fn send_buffer(
        &self,
        buffer: &Arc<Vec<u8>>,
        traffic_type: TrafficType,
    ) -> anyhow::Result<()>;

    async fn send(&self, message: &Message, traffic_type: TrafficType) -> anyhow::Result<()>;

    fn close(&self);
}

#[derive(FromPrimitive, Copy, Clone, Debug)]
pub enum TrafficType {
    Generic,
    /** For bootstrap (asc_pull_ack, asc_pull_req) traffic */
    Bootstrap,
}

pub enum ChannelEnum {
    Tcp(Arc<ChannelTcp>),
    InProc(ChannelInProc),
    Fake(ChannelFake),
}

impl ChannelEnum {
    #[allow(dead_code)]
    pub fn new_null() -> Self {
        Self::new_null_with_channel_id(42)
    }

    #[allow(dead_code)]
    pub(crate) fn new_null_with_channel_id(channel_id: impl Into<ChannelId>) -> Self {
        use rsnano_messages::ProtocolInfo;
        use std::net::Ipv6Addr;

        Self::Fake(ChannelFake::new(
            SystemTime::now(),
            channel_id.into(),
            SocketAddrV6::new(Ipv6Addr::LOCALHOST, 123, 0, 0),
            ProtocolInfo::dev_network(),
        ))
    }

    pub fn max(&self, traffic_type: TrafficType) -> bool {
        match self {
            Self::Tcp(tcp) => tcp.max(traffic_type),
            _ => false,
        }
    }

    pub fn set_peering_endpoint(&self, address: SocketAddrV6) {
        if let Self::Tcp(tcp) = self {
            tcp.set_peering_endpoint(address);
        }
    }
}

impl Deref for ChannelEnum {
    type Target = dyn Channel;

    fn deref(&self) -> &Self::Target {
        match &self {
            ChannelEnum::Tcp(tcp) => tcp,
            ChannelEnum::InProc(inproc) => inproc,
            ChannelEnum::Fake(fake) => fake,
        }
    }
}

#[async_trait]
impl AsyncBufferReader for ChannelEnum {
    async fn read(&self, buffer: &mut [u8], count: usize) -> anyhow::Result<()> {
        self.deref().read(buffer, count).await
    }
}
