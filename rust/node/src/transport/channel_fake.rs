use crate::{
    stats::{DetailType, Direction, StatType, Stats},
    utils::{AsyncRuntime, ErrorCode},
};
use rsnano_core::Account;
use rsnano_messages::{Message, MessageSerializer, ProtocolInfo};
use std::{
    net::SocketAddrV6,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, Weak,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use super::{
    BandwidthLimitType, BufferDropPolicy, Channel, ChannelDirection, ChannelId, ChannelMode,
    OutboundBandwidthLimiter, TrafficType, WriteCallback,
};

pub struct FakeChannelData {
    last_bootstrap_attempt: SystemTime,
    last_packet_received: SystemTime,
    last_packet_sent: SystemTime,
    node_id: Option<Account>,
}

pub struct ChannelFake {
    channel_id: ChannelId,
    async_rt: Weak<AsyncRuntime>,
    channel_mutex: Mutex<FakeChannelData>,
    limiter: Arc<OutboundBandwidthLimiter>,
    stats: Arc<Stats>,
    endpoint: SocketAddrV6,
    closed: AtomicBool,
    protocol: ProtocolInfo,
    message_serializer: Mutex<MessageSerializer>, // TODO remove Mutex!
}

impl ChannelFake {
    pub fn new(
        now: SystemTime,
        channel_id: ChannelId,
        async_rt: &Arc<AsyncRuntime>,
        limiter: Arc<OutboundBandwidthLimiter>,
        stats: Arc<Stats>,
        endpoint: SocketAddrV6,
        protocol: ProtocolInfo,
    ) -> Self {
        Self {
            channel_id,
            async_rt: Arc::downgrade(async_rt),
            channel_mutex: Mutex::new(FakeChannelData {
                last_bootstrap_attempt: UNIX_EPOCH,
                last_packet_received: now,
                last_packet_sent: now,
                node_id: None,
            }),
            limiter,
            stats,
            endpoint,
            closed: AtomicBool::new(false),
            protocol,
            message_serializer: Mutex::new(MessageSerializer::new(protocol)),
        }
    }

    pub fn send_buffer(
        &self,
        buffer_a: &Arc<Vec<u8>>,
        callback_a: Option<WriteCallback>,
        _policy_a: BufferDropPolicy,
        _traffic_type: TrafficType,
    ) {
        let size = buffer_a.len();
        if let Some(cb) = callback_a {
            if let Some(async_rt) = self.async_rt.upgrade() {
                async_rt.post(Box::new(move || {
                    cb(ErrorCode::new(), size);
                }))
            }
        }
    }
}

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

    fn remote_endpoint(&self) -> SocketAddrV6 {
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

    fn send(
        &self,
        message: &Message,
        callback: Option<WriteCallback>,
        drop_policy: BufferDropPolicy,
        traffic_type: TrafficType,
    ) {
        let buffer = {
            let mut serializer = self.message_serializer.lock().unwrap();
            let buffer = serializer.serialize(message);
            Arc::new(Vec::from(buffer)) // TODO don't copy into vec!
        };
        let detail = DetailType::from(message);
        let is_droppable_by_limiter = drop_policy == BufferDropPolicy::Limiter;
        let should_pass = self
            .limiter
            .should_pass(buffer.len(), BandwidthLimitType::from(traffic_type));

        if !is_droppable_by_limiter || should_pass {
            self.send_buffer(&buffer, callback, drop_policy, traffic_type);
            self.stats
                .inc_dir(StatType::Message, detail, Direction::Out);
        } else {
            if let Some(cb) = callback {
                if let Some(async_rt) = self.async_rt.upgrade() {
                    async_rt.post(Box::new(move || {
                        cb(ErrorCode::not_supported(), 0);
                    }))
                }
            }

            self.stats.inc_dir(StatType::Drop, detail, Direction::Out);
        }
    }

    fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
    }
}
