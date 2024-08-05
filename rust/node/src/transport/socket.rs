use super::{
    message_deserializer::AsyncBufferReader, write_queue::WriteQueue, TcpStream, TrafficType,
};
use crate::{stats, utils::into_ipv6_socket_address};
use async_trait::async_trait;
use num_traits::FromPrimitive;
use rsnano_core::utils::{seconds_since_epoch, NULL_ENDPOINT};
use std::{
    net::{Ipv6Addr, SocketAddrV6},
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicU8, AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::time::sleep;
use tracing::debug;

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

pub trait SocketObserver: Send + Sync {
    fn socket_connected(&self, _socket: Arc<Socket>) {}
    fn disconnect_due_to_timeout(&self, _endpoint: SocketAddrV6) {}
    fn connect_error(&self) {}
    fn read_error(&self) {}
    fn read_successful(&self, _len: usize) {}
    fn write_error(&self) {}
    fn write_successful(&self, _len: usize) {}
    fn silent_connection_dropped(&self) {}
    fn inactive_connection_dropped(&self, _direction: ChannelDirection) {}
}

#[derive(Default)]
pub struct NullSocketObserver {}

impl NullSocketObserver {
    pub fn new() -> Self {
        Default::default()
    }
}

impl SocketObserver for NullSocketObserver {}

pub struct Socket {
    pub socket_id: usize,
    /// The other end of the connection
    remote: SocketAddrV6,

    /// the timestamp (in seconds since epoch) of the last time there was successful activity on the socket
    /// activity is any successful connect, send or receive event
    last_completion_time_or_init: AtomicU64,

    /// the timestamp (in seconds since epoch) of the last time there was successful receive on the socket
    /// successful receive includes graceful closing of the socket by the peer (the read succeeds but returns 0 bytes)
    last_receive_time_or_init: AtomicU64,

    default_timeout: AtomicU64,

    /// Duration in seconds of inactivity that causes a socket timeout
    /// activity is any successful connect, send or receive event
    timeout_seconds: AtomicU64,

    idle_timeout: Duration,

    direction: ChannelDirection,
    /// used in real time server sockets, number of seconds of no receive traffic that will cause the socket to timeout
    pub silent_connection_tolerance_time: AtomicU64,

    /// Flag that is set when cleanup decides to close the socket due to timeout.
    /// NOTE: Currently used by tcp_server::timeout() but I suspect that this and tcp_server::timeout() are not needed.
    timed_out: AtomicBool,

    /// Set by close() - completion handlers must check this. This is more reliable than checking
    /// error codes as the OS may have already completed the async operation.
    closed: AtomicBool,

    socket_type: AtomicU8,

    observer: Arc<dyn SocketObserver>,

    write_queue: WriteQueue,
    stream: Arc<TcpStream>,
}

impl Socket {
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst) || self.write_queue.is_closed()
    }

    fn set_last_completion(&self) {
        self.last_completion_time_or_init
            .store(seconds_since_epoch(), std::sync::atomic::Ordering::SeqCst);
    }

    fn set_last_receive_time(&self) {
        self.last_receive_time_or_init
            .store(seconds_since_epoch(), std::sync::atomic::Ordering::SeqCst);
    }

    /// Set the current timeout of the socket.
    ///  timeout occurs when the last socket completion is more than timeout seconds in the past
    ///  timeout always applies, the socket always has a timeout
    ///  to set infinite timeout, use Duration::MAX
    ///  the function checkup() checks for timeout on a regular interval
    pub fn set_timeout(&self, timeout: Duration) {
        self.timeout_seconds
            .store(timeout.as_secs(), Ordering::SeqCst);
    }

    pub fn set_default_timeout(&self) {
        self.set_default_timeout_value(self.default_timeout.load(Ordering::SeqCst));
    }

    pub fn set_default_timeout_value(&self, seconds: u64) {
        self.timeout_seconds.store(seconds, Ordering::SeqCst);
    }

    pub fn close(&self) {
        if !self.closed.swap(true, Ordering::SeqCst) {
            self.set_default_timeout_value(0);
        }
    }

    pub fn mode(&self) -> ChannelMode {
        ChannelMode::from_u8(self.socket_type.load(Ordering::SeqCst)).unwrap()
    }

    pub fn set_mode(&self, socket_type: ChannelMode) {
        self.socket_type.store(socket_type as u8, Ordering::SeqCst);
    }

    pub fn direction(&self) -> ChannelDirection {
        self.direction
    }

    pub fn local_endpoint_v6(&self) -> SocketAddrV6 {
        self.stream
            .local_addr()
            .map(|addr| into_ipv6_socket_address(addr))
            .unwrap_or(SocketAddrV6::new(Ipv6Addr::LOCALHOST, 0, 0, 0))
    }

    pub fn is_realtime_connection(&self) -> bool {
        self.mode() == ChannelMode::Realtime
    }

    const MAX_QUEUE_SIZE: usize = 128;

    pub fn max(&self, traffic_type: TrafficType) -> bool {
        self.write_queue.capacity(traffic_type) <= Self::MAX_QUEUE_SIZE
    }

    pub fn full(&self, traffic_type: TrafficType) -> bool {
        self.write_queue.capacity(traffic_type) == 0
    }

    pub fn is_bootstrap_connection(&self) -> bool {
        self.mode() == ChannelMode::Bootstrap
    }

    pub fn default_timeout_value(&self) -> u64 {
        self.default_timeout.load(Ordering::SeqCst)
    }

    pub fn is_alive(&self) -> bool {
        !self.is_closed()
    }

    pub fn has_timed_out(&self) -> bool {
        self.timed_out.load(Ordering::SeqCst)
    }

    pub fn remote_addr(&self) -> SocketAddrV6 {
        self.remote
    }

    pub async fn read_raw(&self, buffer: &mut [u8], size: usize) -> anyhow::Result<()> {
        if size > buffer.len() {
            return Err(anyhow!("buffer is too small for read count"));
        }

        if self.is_closed() {
            return Err(anyhow!("Tried to read from a closed TcpStream"));
        }

        self.set_default_timeout();

        let mut read = 0;
        loop {
            match self.stream.readable().await {
                Ok(_) => {
                    match self.stream.try_read(&mut buffer[read..size]) {
                        Ok(0) => {
                            self.observer.read_error();
                            return Err(anyhow!("remote side closed the channel"));
                        }
                        Ok(n) => {
                            read += n;
                            if read >= size {
                                self.observer.read_successful(size);
                                self.set_last_completion();
                                self.set_last_receive_time();
                                return Ok(());
                            }
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            continue;
                        }
                        Err(e) => {
                            self.observer.read_error();
                            return Err(e.into());
                        }
                    };
                }
                Err(e) => {
                    self.observer.read_error();
                    return Err(e.into());
                }
            }
        }
    }

    pub async fn write(&self, buffer: &[u8], traffic_type: TrafficType) -> anyhow::Result<()> {
        if self.is_closed() {
            bail!("socket closed");
        }

        let buf_size = buffer.len();

        let result = self
            .write_queue
            .insert(Arc::new(buffer.to_vec()), traffic_type)
            .await;

        if result.is_ok() {
            self.observer.write_successful(buf_size);
            self.set_last_completion();
        } else {
            self.observer.write_error();
            self.close();
        }

        result
    }

    pub fn try_write(&self, buffer: &[u8], traffic_type: TrafficType) {
        if self.is_closed() {
            return;
        }

        let buf_size = buffer.len();

        let (inserted, write_error) = self
            .write_queue
            .try_insert(Arc::new(buffer.to_vec()), traffic_type);

        if inserted {
            self.observer.write_successful(buf_size);
            self.set_last_completion();
        } else if write_error {
            self.observer.write_error();
            self.close();
        }
    }

    async fn ongoing_checkup(&self) {
        loop {
            sleep(Duration::from_secs(2)).await;
            // If the socket is already dead, close just in case, and stop doing checkups
            if !self.is_alive() {
                self.close();
                return;
            }

            let now = seconds_since_epoch();
            let mut condition_to_disconnect = false;

            // if this is a server socket, and no data is received for silent_connection_tolerance_time seconds then disconnect
            if self.direction == ChannelDirection::Inbound
                && (now - self.last_receive_time_or_init.load(Ordering::SeqCst))
                    > self.silent_connection_tolerance_time.load(Ordering::SeqCst)
            {
                self.observer.silent_connection_dropped();
                condition_to_disconnect = true;
            }

            // if there is no activity for timeout seconds then disconnect
            if (now - self.last_completion_time_or_init.load(Ordering::SeqCst))
                > self.timeout_seconds.load(Ordering::SeqCst)
            {
                self.observer.inactive_connection_dropped(self.direction);
                condition_to_disconnect = true;
            }

            if condition_to_disconnect {
                self.observer.disconnect_due_to_timeout(self.remote);
                self.timed_out.store(true, Ordering::SeqCst);
                self.close();
            }
        }
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        self.close();
        let alive = LIVE_SOCKETS.fetch_sub(1, Ordering::Relaxed) - 1;
        debug!(socket_id = self.socket_id, alive, "Socket dropped");
    }
}

#[async_trait]
impl AsyncBufferReader for Arc<Socket> {
    async fn read(&self, buffer: &mut [u8], count: usize) -> anyhow::Result<()> {
        self.read_raw(buffer, count).await
    }
}

pub struct SocketBuilder {
    direction: ChannelDirection,
    default_timeout: Duration,
    silent_connection_tolerance_time: Duration,
    idle_timeout: Duration,
    observer: Option<Arc<dyn SocketObserver>>,
    max_write_queue_len: usize,
}

static NEXT_SOCKET_ID: AtomicUsize = AtomicUsize::new(0);
static LIVE_SOCKETS: AtomicUsize = AtomicUsize::new(0);

pub fn alive_sockets() -> usize {
    LIVE_SOCKETS.load(Ordering::Relaxed)
}

impl SocketBuilder {
    pub fn new(direction: ChannelDirection) -> Self {
        Self {
            direction,
            default_timeout: Duration::from_secs(15),
            silent_connection_tolerance_time: Duration::from_secs(120),
            idle_timeout: Duration::from_secs(120),
            observer: None,
            max_write_queue_len: Socket::MAX_QUEUE_SIZE,
        }
    }

    pub fn default_timeout(mut self, timeout: Duration) -> Self {
        self.default_timeout = timeout;
        self
    }

    pub fn silent_connection_tolerance_time(mut self, timeout: Duration) -> Self {
        self.silent_connection_tolerance_time = timeout;
        self
    }

    pub fn idle_timeout(mut self, timeout: Duration) -> Self {
        self.idle_timeout = timeout;
        self
    }

    pub fn observer(mut self, observer: Arc<dyn SocketObserver>) -> Self {
        self.observer = Some(observer);
        self
    }

    pub fn max_write_queue_len(mut self, max_len: usize) -> Self {
        self.max_write_queue_len = max_len;
        self
    }

    pub async fn finish(self, stream: TcpStream) -> Arc<Socket> {
        let socket_id = NEXT_SOCKET_ID.fetch_add(1, Ordering::Relaxed);
        let alive = LIVE_SOCKETS.fetch_add(1, Ordering::Relaxed) + 1;
        debug!(socket_id, alive, "Creating socket");

        let remote = stream
            .peer_addr()
            .map(into_ipv6_socket_address)
            .unwrap_or(NULL_ENDPOINT);

        let observer = self
            .observer
            .unwrap_or_else(|| Arc::new(NullSocketObserver::new()));

        let (write_queue, mut receiver) = WriteQueue::new(self.max_write_queue_len);
        let stream = Arc::new(stream);
        let stream_l = stream.clone();
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

        let socket = Arc::new({
            Socket {
                socket_id,
                remote,
                last_completion_time_or_init: AtomicU64::new(seconds_since_epoch()),
                last_receive_time_or_init: AtomicU64::new(seconds_since_epoch()),
                default_timeout: AtomicU64::new(self.default_timeout.as_secs()),
                timeout_seconds: AtomicU64::new(u64::MAX),
                idle_timeout: self.idle_timeout,
                direction: self.direction,
                silent_connection_tolerance_time: AtomicU64::new(
                    self.silent_connection_tolerance_time.as_secs(),
                ),
                timed_out: AtomicBool::new(false),
                closed: AtomicBool::new(false),
                socket_type: AtomicU8::new(ChannelMode::Undefined as u8),
                observer,
                write_queue,
                stream,
            }
        });
        socket.set_default_timeout();

        let socket_l = socket.clone();
        tokio::spawn(async move { socket_l.ongoing_checkup().await });
        socket
    }
}
