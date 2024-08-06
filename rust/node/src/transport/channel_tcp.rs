use super::{
    write_queue::{WriteQueue, WriteQueueReceiver},
    AsyncBufferReader, BufferDropPolicy, Channel, ChannelDirection, ChannelId, ChannelMode,
    OutboundBandwidthLimiter, TcpStream, TrafficType,
};
use crate::{
    stats::{DetailType, Direction, StatType, Stats},
    utils::{into_ipv6_socket_address, ipv4_address_or_ipv6_subnet, map_address_to_subnetwork},
};
use async_trait::async_trait;
use num::FromPrimitive;
use rsnano_core::{
    utils::{seconds_since_epoch, NULL_ENDPOINT},
    Account,
};
use rsnano_messages::{Message, MessageSerializer, ProtocolInfo};
use std::{
    fmt::Display,
    net::{Ipv6Addr, SocketAddrV6},
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering},
        Arc, Mutex,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::time::sleep;
use tracing::{debug, trace};

pub struct TcpChannelData {
    last_bootstrap_attempt: SystemTime,
    last_packet_received: SystemTime,
    last_packet_sent: SystemTime,
    node_id: Option<Account>,
    peering_endpoint: Option<SocketAddrV6>,
}

/// Default timeout in seconds
const DEFAULT_TIMEOUT: u64 = 120;

pub struct ChannelTcp {
    channel_id: ChannelId,
    channel_mutex: Mutex<TcpChannelData>,
    network_version: AtomicU8,
    limiter: Arc<OutboundBandwidthLimiter>,
    message_serializer: Mutex<MessageSerializer>, // TODO remove mutex
    stats: Arc<Stats>,

    /// The other end of the connection
    remote: SocketAddrV6,

    /// the timestamp (in seconds since epoch) of the last time there was successful activity on the socket
    last_activity: AtomicU64,

    /// Duration in seconds of inactivity that causes a socket timeout
    /// activity is any successful connect, send or receive event
    timeout_seconds: AtomicU64,

    direction: ChannelDirection,

    /// Flag that is set when cleanup decides to close the socket due to timeout.
    /// NOTE: Currently used by tcp_server::timeout() but I suspect that this and tcp_server::timeout() are not needed.
    timed_out: AtomicBool,

    /// Set by close() - completion handlers must check this. This is more reliable than checking
    /// error codes as the OS may have already completed the async operation.
    closed: AtomicBool,

    socket_type: AtomicU8,

    write_queue: WriteQueue,
    stream: Arc<TcpStream>,
    ignore_closed_write_queue: bool,
}

impl ChannelTcp {
    const MAX_QUEUE_SIZE: usize = 128;

    fn new(
        channel_id: ChannelId,
        stream: Arc<TcpStream>,
        direction: ChannelDirection,
        protocol: ProtocolInfo,
        stats: Arc<Stats>,
        limiter: Arc<OutboundBandwidthLimiter>,
    ) -> (Self, WriteQueueReceiver) {
        let remote = stream
            .peer_addr()
            .map(into_ipv6_socket_address)
            .unwrap_or(NULL_ENDPOINT);

        let (write_queue, receiver) = WriteQueue::new(Self::MAX_QUEUE_SIZE);

        let peering_endpoint = match direction {
            ChannelDirection::Inbound => None,
            ChannelDirection::Outbound => Some(remote),
        };

        let now = SystemTime::now();
        let channel = Self {
            channel_id,
            channel_mutex: Mutex::new(TcpChannelData {
                last_bootstrap_attempt: UNIX_EPOCH,
                last_packet_received: now,
                last_packet_sent: now,
                node_id: None,
                peering_endpoint,
            }),
            network_version: AtomicU8::new(protocol.version_using),
            limiter,
            message_serializer: Mutex::new(MessageSerializer::new(protocol)),
            stats,
            remote,
            last_activity: AtomicU64::new(seconds_since_epoch()),
            timeout_seconds: AtomicU64::new(DEFAULT_TIMEOUT),
            direction,
            timed_out: AtomicBool::new(false),
            closed: AtomicBool::new(false),
            socket_type: AtomicU8::new(ChannelMode::Undefined as u8),
            write_queue,
            stream,
            ignore_closed_write_queue: false,
        };

        (channel, receiver)
    }

    pub fn new_null() -> Self {
        let (mut channel, _receiver) = Self::new(
            ChannelId::from(42),
            Arc::new(TcpStream::new_null()),
            ChannelDirection::Inbound,
            ProtocolInfo::default(),
            Arc::new(Stats::default()),
            Arc::new(OutboundBandwidthLimiter::default()),
        );
        // We drop the write queue receiver, so the channel would be dead immediately.
        channel.ignore_closed_write_queue = true;
        channel
    }

    pub async fn create(
        channel_id: ChannelId,
        stream: TcpStream,
        direction: ChannelDirection,
        protocol: ProtocolInfo,
        stats: Arc<Stats>,
        limiter: Arc<OutboundBandwidthLimiter>,
    ) -> Arc<Self> {
        let stream = Arc::new(stream);
        let stream_l = stream.clone();
        let (channel, mut receiver) =
            Self::new(channel_id, stream, direction, protocol, stats, limiter);
        //
        // process write queue:
        tokio::spawn(async move {
            while let Some(entry) = receiver.pop().await {
                let mut written = 0;
                let buffer = &entry.buffer;
                loop {
                    match stream_l.writable().await {
                        Ok(()) => match stream_l.try_write(&buffer[written..]) {
                            Ok(n) => {
                                written += n;
                                if written >= buffer.len() {
                                    break;
                                }
                            }
                            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                continue;
                            }
                            Err(_) => {
                                break;
                            }
                        },
                        Err(_) => {
                            break;
                        }
                    }
                }
            }
        });

        let channel = Arc::new(channel);
        let channel_l = channel.clone();
        tokio::spawn(async move { channel_l.ongoing_checkup().await });
        channel
    }

    pub(crate) fn set_peering_endpoint(&self, address: SocketAddrV6) {
        let mut lock = self.channel_mutex.lock().unwrap();
        lock.peering_endpoint = Some(address);
    }

    pub(crate) fn max(&self, traffic_type: TrafficType) -> bool {
        self.write_queue.capacity(traffic_type) <= Self::MAX_QUEUE_SIZE
    }

    fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
            || (!self.ignore_closed_write_queue && self.write_queue.is_closed())
    }

    fn is_alive_impl(&self) -> bool {
        !self.is_closed()
    }

    fn update_last_activity(&self) {
        self.last_activity
            .store(seconds_since_epoch(), Ordering::Relaxed);
    }

    fn set_timeout(&self, seconds: u64) {
        self.timeout_seconds.store(seconds, Ordering::Relaxed);
    }

    async fn ongoing_checkup(&self) {
        loop {
            sleep(Duration::from_secs(2)).await;
            // If the socket is already dead, close just in case, and stop doing checkups
            if !self.is_alive_impl() {
                debug!("Closing socket because it was dead ({})", self.remote);
                self.close_internal();
                return;
            }

            let now = seconds_since_epoch();
            let mut condition_to_disconnect = false;

            // if there is no activity for timeout seconds then disconnect
            if (now - self.last_activity.load(Ordering::Relaxed))
                > self.timeout_seconds.load(Ordering::Relaxed)
            {
                self.stats.inc_dir(
                    StatType::Tcp,
                    DetailType::TcpIoTimeoutDrop,
                    if self.direction == ChannelDirection::Inbound {
                        Direction::In
                    } else {
                        Direction::Out
                    },
                );
                condition_to_disconnect = true;
            }

            if condition_to_disconnect {
                debug!("Closing socket due to timeout ({})", self.remote);
                self.timed_out.store(true, Ordering::SeqCst);
                self.close_internal();
            }
        }
    }

    fn close_internal(&self) {
        if !self.closed.swap(true, Ordering::SeqCst) {
            self.set_timeout(0);
        }
    }

    async fn read_raw(&self, buffer: &mut [u8], size: usize) -> anyhow::Result<()> {
        if size > buffer.len() {
            return Err(anyhow!("buffer is too small for read count"));
        }

        if self.is_closed() {
            return Err(anyhow!("Tried to read from a closed TcpStream"));
        }

        let mut read = 0;
        loop {
            match self.stream.readable().await {
                Ok(_) => {
                    match self.stream.try_read(&mut buffer[read..size]) {
                        Ok(0) => {
                            self.stats.inc_dir(
                                StatType::Tcp,
                                DetailType::TcpReadError,
                                Direction::In,
                            );
                            return Err(anyhow!("remote side closed the channel"));
                        }
                        Ok(n) => {
                            read += n;
                            if read >= size {
                                self.stats.add_dir(
                                    StatType::TrafficTcp,
                                    DetailType::All,
                                    Direction::In,
                                    size as u64,
                                );
                                self.update_last_activity();
                                return Ok(());
                            }
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            continue;
                        }
                        Err(e) => {
                            self.stats.inc_dir(
                                StatType::Tcp,
                                DetailType::TcpReadError,
                                Direction::In,
                            );
                            return Err(e.into());
                        }
                    };
                }
                Err(e) => {
                    self.stats
                        .inc_dir(StatType::Tcp, DetailType::TcpReadError, Direction::In);
                    return Err(e.into());
                }
            }
        }
    }

    async fn write(&self, buffer: &[u8], traffic_type: TrafficType) -> anyhow::Result<()> {
        if self.is_closed() {
            bail!("socket closed");
        }

        let buf_size = buffer.len();

        let result = self
            .write_queue
            .insert(Arc::new(buffer.to_vec()), traffic_type)
            .await;

        if result.is_ok() {
            self.stats.add_dir_aggregate(
                StatType::TrafficTcp,
                DetailType::All,
                Direction::Out,
                buf_size as u64,
            );
            self.update_last_activity();
        } else {
            self.stats
                .inc_dir(StatType::Tcp, DetailType::TcpWriteError, Direction::In);
            debug!("Closing socket after write error: {}", self.remote);
            self.close_internal();
        }

        result
    }

    fn try_write(&self, buffer: &[u8], traffic_type: TrafficType) {
        if self.is_closed() {
            return;
        }

        let buf_size = buffer.len();

        let (inserted, write_error) = self
            .write_queue
            .try_insert(Arc::new(buffer.to_vec()), traffic_type);

        if inserted {
            self.stats.add_dir_aggregate(
                StatType::TrafficTcp,
                DetailType::All,
                Direction::Out,
                buf_size as u64,
            );
            self.update_last_activity();
        } else if write_error {
            self.stats
                .inc_dir(StatType::Tcp, DetailType::TcpWriteError, Direction::In);
            self.close_internal();
            debug!("Closing socket after write error: {}", self.remote);
        }
    }
}

impl Display for ChannelTcp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.remote.fmt(f)
    }
}

#[async_trait]
impl Channel for Arc<ChannelTcp> {
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
        self.is_alive_impl()
    }

    fn get_type(&self) -> super::TransportType {
        super::TransportType::Tcp
    }

    fn local_addr(&self) -> SocketAddrV6 {
        self.stream
            .local_addr()
            .map(|addr| into_ipv6_socket_address(addr))
            .unwrap_or(SocketAddrV6::new(Ipv6Addr::LOCALHOST, 0, 0, 0))
    }

    fn remote_addr(&self) -> SocketAddrV6 {
        self.remote
    }

    fn peering_endpoint(&self) -> Option<SocketAddrV6> {
        self.channel_mutex.lock().unwrap().peering_endpoint
    }

    fn network_version(&self) -> u8 {
        self.network_version.load(Ordering::Relaxed)
    }

    fn direction(&self) -> ChannelDirection {
        self.direction
    }

    fn mode(&self) -> ChannelMode {
        FromPrimitive::from_u8(self.socket_type.load(Ordering::SeqCst)).unwrap()
    }

    fn set_mode(&self, mode: ChannelMode) {
        self.socket_type.store(mode as u8, Ordering::SeqCst);
    }

    fn set_timeout(&self, timeout: Duration) {
        self.timeout_seconds
            .store(timeout.as_secs(), Ordering::SeqCst);
    }

    fn try_send(
        &self,
        message: &Message,
        drop_policy: BufferDropPolicy,
        traffic_type: TrafficType,
    ) {
        let buffer = {
            let mut serializer = self.message_serializer.lock().unwrap();
            let buffer = serializer.serialize(message);
            Arc::new(Vec::from(buffer)) // TODO don't copy into vec. Pass slice directly
        };

        let is_droppable_by_limiter = drop_policy == BufferDropPolicy::Limiter;
        let should_pass = self.limiter.should_pass(buffer.len(), traffic_type.into());
        if !is_droppable_by_limiter || should_pass {
            self.try_write(&buffer, traffic_type);
            self.stats
                .inc_dir_aggregate(StatType::Message, message.into(), Direction::Out);
            trace!(channel_id = %self.channel_id, message = ?message, "Message sent");
        } else {
            let detail_type = message.into();
            self.stats
                .inc_dir_aggregate(StatType::Drop, detail_type, Direction::Out);
            trace!(channel_id = %self.channel_id, message = ?message, "Message dropped");
        }
    }

    async fn send_buffer(&self, buffer: &[u8], traffic_type: TrafficType) -> anyhow::Result<()> {
        while !self.limiter.should_pass(buffer.len(), traffic_type.into()) {
            // TODO: better implementation
            sleep(Duration::from_millis(20)).await;
        }

        self.write(buffer, traffic_type).await?;
        self.channel_mutex.lock().unwrap().last_packet_sent = SystemTime::now();
        Ok(())
    }

    async fn send(&self, message: &Message, traffic_type: TrafficType) -> anyhow::Result<()> {
        let buffer = {
            let mut serializer = self.message_serializer.lock().unwrap();
            let buffer = serializer.serialize(message);
            Arc::new(Vec::from(buffer)) // TODO don't copy into vec. Pass slice directly
        };
        self.send_buffer(&buffer, traffic_type).await?;
        self.stats
            .inc_dir_aggregate(StatType::Message, message.into(), Direction::Out);
        trace!(channel_id = %self.channel_id, message = ?message, "Message sent");
        Ok(())
    }

    fn close(&self) {
        self.close_internal();
    }

    fn ipv4_address_or_ipv6_subnet(&self) -> Ipv6Addr {
        ipv4_address_or_ipv6_subnet(&self.remote_addr().ip())
    }

    fn subnetwork(&self) -> Ipv6Addr {
        map_address_to_subnetwork(self.remote_addr().ip())
    }
}

impl Drop for ChannelTcp {
    fn drop(&mut self) {
        self.close_internal();
    }
}

#[async_trait]
impl AsyncBufferReader for Arc<ChannelTcp> {
    async fn read(&self, buffer: &mut [u8], count: usize) -> anyhow::Result<()> {
        self.read_raw(buffer, count).await
    }
}
