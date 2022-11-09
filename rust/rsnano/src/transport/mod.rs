mod bandwidth_limiter;
mod channel_fake;
mod channel_inproc;
mod channel_tcp;
mod channel_udp;
mod message_deserializer;
mod network_filter;
mod peer_exclusion;
mod socket;
mod syn_cookies;
mod tcp_channels;
mod tcp_message_manager;
mod tcp_server;
mod token_bucket;

pub use bandwidth_limiter::{
    BandwidthLimitType, BandwidthLimiter, OutboundBandwidthLimiter, OutboundBandwidthLimiterConfig,
};
pub use channel_fake::ChannelFake;
pub use channel_inproc::ChannelInProc;
pub use channel_tcp::{ChannelTcp, ChannelTcpObserver, TcpChannelData};
pub use channel_udp::ChannelUdp;
pub use message_deserializer::{MessageDeserializer, MessageDeserializerExt, ParseStatus};
pub use network_filter::NetworkFilter;
pub use peer_exclusion::PeerExclusion;
pub use socket::*;
pub use syn_cookies::SynCookies;
pub use tcp_channels::TcpChannels;
pub use tcp_message_manager::{TcpMessageItem, TcpMessageManager};
pub use tcp_server::{
    BootstrapMessageVisitor, HandshakeMessageVisitor, HandshakeMessageVisitorImpl,
    RealtimeMessageVisitor, RealtimeMessageVisitorImpl, RequestResponseVisitorFactory, TcpServer,
    TcpServerExt, TcpServerObserver,
};
use token_bucket::TokenBucket;

use crate::core::Account;

pub trait Channel {
    fn is_temporary(&self) -> bool;
    fn set_temporary(&self, temporary: bool);
    fn get_last_bootstrap_attempt(&self) -> u64;
    fn set_last_bootstrap_attempt(&self, instant: u64);
    fn get_last_packet_received(&self) -> u64;
    fn set_last_packet_received(&self, instant: u64);
    fn get_last_packet_sent(&self) -> u64;
    fn set_last_packet_sent(&self, instant: u64);
    fn get_node_id(&self) -> Option<Account>;
    fn set_node_id(&self, id: Account);
}
