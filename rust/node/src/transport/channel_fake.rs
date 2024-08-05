use super::{
    AsyncBufferReader, BufferDropPolicy, Channel, ChannelDirection, ChannelId, ChannelMode,
    TrafficType,
};
use async_trait::async_trait;
use rsnano_core::Account;
use rsnano_messages::{Message, ProtocolInfo};
use std::{
    net::{Ipv6Addr, SocketAddrV6},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub struct FakeChannelData {
    last_bootstrap_attempt: SystemTime,
    last_packet_received: SystemTime,
    last_packet_sent: SystemTime,
    node_id: Option<Account>,
}

pub struct ChannelFake {
    channel_id: ChannelId,
    channel_mutex: Mutex<FakeChannelData>,
    endpoint: SocketAddrV6,
    closed: AtomicBool,
    protocol: ProtocolInfo,
}

impl ChannelFake {
    pub fn new(
        now: SystemTime,
        channel_id: ChannelId,
        endpoint: SocketAddrV6,
        protocol: ProtocolInfo,
    ) -> Self {
        Self {
            channel_id,
            channel_mutex: Mutex::new(FakeChannelData {
                last_bootstrap_attempt: UNIX_EPOCH,
                last_packet_received: now,
                last_packet_sent: now,
                node_id: None,
            }),
            endpoint,
            closed: AtomicBool::new(false),
            protocol,
        }
    }
}

#[async_trait]
impl Channel for ChannelFake {
    fn channel_id(&self) -> ChannelId {
        self.channel_id
    }

    fn get_last_bootstrap_attempt(&self) -> SystemTime {
        self.channel_mutex.lock().unwrap().last_bootstrap_attempt
    }

    fn set_last_bootstrap_attempt(&self, time: SystemTime) {
        self.channel_mutex.lock().unwrap().last_bootstrap_attempt = time;
    }

    fn get_last_packet_received(&self) -> SystemTime {
        self.channel_mutex.lock().unwrap().last_packet_received
    }

    fn set_last_packet_received(&self, instant: SystemTime) {
        self.channel_mutex.lock().unwrap().last_packet_received = instant;
    }

    fn get_last_packet_sent(&self) -> SystemTime {
        self.channel_mutex.lock().unwrap().last_packet_sent
    }

    fn set_last_packet_sent(&self, instant: SystemTime) {
        self.channel_mutex.lock().unwrap().last_packet_sent = instant;
    }

    fn get_node_id(&self) -> Option<Account> {
        self.channel_mutex.lock().unwrap().node_id
    }

    fn set_node_id(&self, id: Account) {
        self.channel_mutex.lock().unwrap().node_id = Some(id);
    }

    fn is_alive(&self) -> bool {
        !self.closed.load(Ordering::SeqCst)
    }

    fn get_type(&self) -> super::TransportType {
        super::TransportType::Fake
    }

    fn local_addr(&self) -> SocketAddrV6 {
        self.endpoint
    }

    fn remote_addr(&self) -> SocketAddrV6 {
        self.endpoint
    }

    fn peering_endpoint(&self) -> Option<SocketAddrV6> {
        Some(self.endpoint)
    }

    fn network_version(&self) -> u8 {
        self.protocol.version_using
    }

    fn direction(&self) -> ChannelDirection {
        ChannelDirection::Inbound
    }

    fn mode(&self) -> ChannelMode {
        ChannelMode::Realtime
    }

    fn set_mode(&self, _mode: ChannelMode) {}

    fn set_timeout(&self, _timeout: Duration) {}

    fn try_send(
        &self,
        _message: &Message,
        _drop_policy: BufferDropPolicy,
        _traffic_type: TrafficType,
    ) {
    }

    async fn send_buffer(&self, _buffer: &[u8], _traffic_type: TrafficType) -> anyhow::Result<()> {
        Ok(())
    }

    async fn send(&self, _message: &Message, _traffic_type: TrafficType) -> anyhow::Result<()> {
        Ok(())
    }

    fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
    }

    fn ipv4_address_or_ipv6_subnet(&self) -> Ipv6Addr {
        Ipv6Addr::UNSPECIFIED
    }

    fn subnetwork(&self) -> Ipv6Addr {
        Ipv6Addr::UNSPECIFIED
    }
}

#[async_trait]
impl AsyncBufferReader for ChannelFake {
    async fn read(&self, _buffer: &mut [u8], _count: usize) -> anyhow::Result<()> {
        Err(anyhow!("AsyncBufferReader not implemented for ChannelFake"))
    }
}
