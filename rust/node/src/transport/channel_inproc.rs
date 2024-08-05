use std::{
    net::{Ipv6Addr, SocketAddrV6},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, Weak,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;
use rsnano_core::Account;
use rsnano_messages::{DeserializedMessage, Message, MessageSerializer, ParseMessageError};
use tokio::task::spawn_blocking;

use crate::{
    config::NetworkConstants,
    stats::{Direction, StatType, Stats},
    utils::{AsyncRuntime, ErrorCode},
};

use super::{
    message_deserializer::{AsyncBufferReader, MessageDeserializer},
    BufferDropPolicy, Channel, ChannelDirection, ChannelEnum, ChannelId, ChannelMode,
    NetworkFilter, OutboundBandwidthLimiter, TrafficType,
};

pub struct InProcChannelData {
    last_bootstrap_attempt: SystemTime,
    last_packet_received: SystemTime,
    last_packet_sent: SystemTime,
    node_id: Option<Account>,
}

pub type InboundCallback = Arc<dyn Fn(DeserializedMessage, Arc<ChannelEnum>) + Send + Sync>;

pub struct ChannelInProc {
    channel_id: ChannelId,
    channel_mutex: Mutex<InProcChannelData>,
    network_constants: NetworkConstants,
    network_filter: Arc<NetworkFilter>,
    stats: Arc<Stats>,
    limiter: Arc<OutboundBandwidthLimiter>,
    source_inbound: InboundCallback,
    destination_inbound: InboundCallback,
    async_rt: Weak<AsyncRuntime>,
    pub local_endpoint: SocketAddrV6,
    remote_endpoint: SocketAddrV6,
    source_node_id: Account,
    destination_node_id: Account,
    message_serializer: Mutex<MessageSerializer>, // TODO remove Mutex!
}

impl ChannelInProc {
    pub fn new(
        channel_id: ChannelId,
        now: SystemTime,
        network_constants: NetworkConstants,
        network_filter: Arc<NetworkFilter>,
        stats: Arc<Stats>,
        limiter: Arc<OutboundBandwidthLimiter>,
        source_inbound: InboundCallback,
        destination_inbound: InboundCallback,
        async_rt: &Arc<AsyncRuntime>,
        local_endpoint: SocketAddrV6,
        remote_endpoint: SocketAddrV6,
        source_node_id: Account,
        destination_node_id: Account,
    ) -> Self {
        Self {
            channel_id,
            channel_mutex: Mutex::new(InProcChannelData {
                last_bootstrap_attempt: UNIX_EPOCH,
                last_packet_received: now,
                last_packet_sent: now,
                node_id: Some(source_node_id),
            }),
            message_serializer: Mutex::new(MessageSerializer::new(
                network_constants.protocol_info(),
            )),
            network_constants,
            network_filter,
            stats,
            limiter,
            source_inbound,
            destination_inbound,
            async_rt: Arc::downgrade(async_rt),
            local_endpoint,
            remote_endpoint,
            source_node_id,
            destination_node_id,
        }
    }

    fn send_buffer_2(&self, buffer: &[u8]) {
        let stats = self.stats.clone();
        let network_constants = self.network_constants.clone();
        let limiter = self.limiter.clone();
        let source_inbound = self.source_inbound.clone();
        let destination_inbound = self.destination_inbound.clone();
        let source_endpoint = self.local_endpoint;
        let destination_endpoint = self.remote_endpoint;
        let source_node_id = self.source_node_id;
        let destination_node_id = self.destination_node_id;
        let async_rt = self.async_rt.clone();

        let callback_wrapper = Box::new(move |ec: ErrorCode, msg: Option<DeserializedMessage>| {
            if ec.is_err() {
                return;
            }
            let Some(async_rt) = async_rt.upgrade() else {
                return;
            };
            let Some(msg) = msg else {
                return;
            };
            let filter = Arc::new(NetworkFilter::new(100000));
            // we create a temporary channel for the reply path, in case the receiver of the message wants to reply
            let remote_channel = Arc::new(ChannelEnum::InProc(ChannelInProc::new(
                1.into(),
                SystemTime::now(),
                network_constants.clone(),
                filter,
                stats.clone(),
                limiter,
                source_inbound,
                destination_inbound.clone(),
                &async_rt,
                source_endpoint,
                destination_endpoint,
                source_node_id,
                destination_node_id,
            )));

            // process message
            {
                stats.inc_dir(
                    StatType::Message,
                    msg.message.message_type().into(),
                    Direction::In,
                );

                destination_inbound(msg, remote_channel);
            }
        });

        self.send_buffer_impl(buffer, callback_wrapper);
    }

    fn send_buffer_impl(
        &self,
        buffer: &[u8],
        callback_msg: Box<dyn FnOnce(ErrorCode, Option<DeserializedMessage>) + Send>,
    ) {
        if let Some(rt) = self.async_rt.upgrade() {
            let mut message_deserializer = MessageDeserializer::new(
                self.network_constants.protocol_info(),
                self.network_constants.work.clone(),
                Arc::clone(&self.network_filter),
                VecBufferReader::new(buffer.to_vec()),
            );

            rt.tokio.spawn(async move {
                let result = message_deserializer.read().await;
                spawn_blocking(move || match result {
                    Ok(msg) => callback_msg(ErrorCode::new(), Some(msg)),
                    Err(ParseMessageError::DuplicatePublishMessage) => {
                        callback_msg(ErrorCode::new(), None)
                    }
                    Err(ParseMessageError::InsufficientWork) => {
                        callback_msg(ErrorCode::new(), None)
                    }
                    Err(_) => callback_msg(ErrorCode::fault(), None),
                });
            });
        }
    }
}

pub struct VecBufferReader {
    buffer: Vec<u8>,
    position: AtomicUsize,
}

impl VecBufferReader {
    pub fn new(buffer: Vec<u8>) -> Self {
        Self {
            buffer,
            position: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl AsyncBufferReader for VecBufferReader {
    async fn read(&self, buffer: &mut [u8], count: usize) -> anyhow::Result<()> {
        let pos = self.position.load(Ordering::SeqCst);
        if count > self.buffer.len() - pos {
            bail!("no more data to read");
        }
        buffer[..count].copy_from_slice(&self.buffer[pos..pos + count]);
        self.position.store(pos + count, Ordering::SeqCst);
        Ok(())
    }
}

#[async_trait]
impl Channel for ChannelInProc {
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
        true
    }

    fn get_type(&self) -> super::TransportType {
        super::TransportType::Loopback
    }

    fn remote_addr(&self) -> SocketAddrV6 {
        self.remote_endpoint
    }

    fn peering_endpoint(&self) -> Option<SocketAddrV6> {
        Some(self.remote_endpoint)
    }

    fn network_version(&self) -> u8 {
        self.network_constants.protocol_version
    }

    fn direction(&self) -> ChannelDirection {
        ChannelDirection::Inbound
    }

    fn mode(&self) -> ChannelMode {
        ChannelMode::Realtime
    }

    fn set_mode(&self, _mode: ChannelMode) {}

    fn try_send(
        &self,
        message: &Message,
        _drop_policy: BufferDropPolicy,
        _traffic_type: TrafficType,
    ) {
        let buffer = {
            let mut serializer = self.message_serializer.lock().unwrap();
            let buffer = serializer.serialize(message);
            Vec::from(buffer)
        };
        self.send_buffer_2(&buffer);
    }

    async fn send_buffer(&self, buffer: &[u8], _traffic_type: TrafficType) -> anyhow::Result<()> {
        self.send_buffer_2(buffer);
        Ok(())
    }

    async fn send(&self, message: &Message, _traffic_type: TrafficType) -> anyhow::Result<()> {
        let buffer = {
            let mut serializer = self.message_serializer.lock().unwrap();
            let buffer = serializer.serialize(message);
            Arc::new(Vec::from(buffer)) // TODO don't copy buffer
        };
        self.send_buffer_2(&buffer);
        Ok(())
    }

    fn close(&self) {
        // Can't be closed
    }

    fn local_addr(&self) -> SocketAddrV6 {
        self.local_endpoint
    }

    fn set_timeout(&self, _timeout: Duration) {}

    fn ipv4_address_or_ipv6_subnet(&self) -> Ipv6Addr {
        Ipv6Addr::UNSPECIFIED
    }

    fn subnetwork(&self) -> Ipv6Addr {
        Ipv6Addr::UNSPECIFIED
    }
}

#[async_trait]
impl AsyncBufferReader for ChannelInProc {
    async fn read(&self, _buffer: &mut [u8], _count: usize) -> anyhow::Result<()> {
        Err(anyhow!(
            "AsyncBufferReader not implemented for ChannelInProc "
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn empty_vec() {
        let reader = VecBufferReader::new(Vec::new());
        let mut buffer = vec![0u8; 3];
        let result = reader.read(&mut buffer, 1).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn read_one_byte() {
        let reader = VecBufferReader::new(vec![42]);
        let mut buffer = vec![0u8; 1];
        let result = reader.read(&mut buffer, 1).await;
        assert!(result.is_ok());
        assert_eq!(buffer[0], 42);
    }

    #[tokio::test]
    async fn multiple_reads() {
        let reader = VecBufferReader::new(vec![1, 2, 3, 4, 5]);
        let mut buffer = vec![0u8; 2];
        reader.read(&mut buffer, 1).await.unwrap();
        assert_eq!(buffer[0], 1);

        reader.read(&mut buffer, 2).await.unwrap();
        assert_eq!(buffer[0], 2);
        assert_eq!(buffer[1], 3);

        reader.read(&mut buffer, 2).await.unwrap();
        assert_eq!(buffer[0], 4);
        assert_eq!(buffer[1], 5);

        assert!(reader.read(&mut buffer, 1).await.is_err());
    }
}
