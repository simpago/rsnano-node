use super::{
    message_deserializer::AsyncBufferReader,
    write_queue::{WriteCallback, WriteQueue},
    TcpStream, TrafficType,
};
use crate::{
    stats,
    utils::{into_ipv6_socket_address, AsyncRuntime, ErrorCode, ThreadPool, ThreadPoolImpl},
};
use async_trait::async_trait;
use num_traits::FromPrimitive;
use rsnano_core::utils::{seconds_since_epoch, NULL_ENDPOINT};
use std::{
    net::{Ipv6Addr, SocketAddrV6},
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicU8, AtomicUsize, Ordering},
        Arc, Mutex, Weak,
    },
    time::Duration,
};
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

    thread_pool: Arc<dyn ThreadPool>,
    direction: ChannelDirection,
    /// used in real time server sockets, number of seconds of no receive traffic that will cause the socket to timeout
    pub silent_connection_tolerance_time: AtomicU64,

    /// Flag that is set when cleanup decides to close the socket due to timeout.
    /// NOTE: Currently used by tcp_server::timeout() but I suspect that this and tcp_server::timeout() are not needed.
    timed_out: AtomicBool,

    /// Set by close() - completion handlers must check this. This is more reliable than checking
    /// error codes as the OS may have already completed the async operation.
    closed: AtomicBool,

    /// Updated only from strand, but stored as atomic so it can be read from outside
    write_in_progress: AtomicBool,

    socket_type: AtomicU8,

    observer: Arc<dyn SocketObserver>,

    send_queue: WriteQueue,
    runtime: Weak<AsyncRuntime>,
    current_action: Mutex<Option<Box<dyn Fn() + Send + Sync>>>,
    stream: Arc<TcpStream>,
}

impl Socket {
    pub fn new_null() -> Arc<Socket> {
        let thread_pool = Arc::new(ThreadPoolImpl::new_null());
        SocketBuilder::new(ChannelDirection::Outbound, thread_pool, Weak::new())
            .finish(TcpStream::new_null())
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
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

    pub fn close_internal(&self) {
        if !self.closed.swap(true, Ordering::SeqCst) {
            self.send_queue.clear();
            self.set_default_timeout_value(0);

            if let Some(cb) = self.current_action.lock().unwrap().take() {
                cb();
            }
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
        self.send_queue.size(traffic_type) >= Self::MAX_QUEUE_SIZE
    }

    pub fn full(&self, traffic_type: TrafficType) -> bool {
        self.send_queue.size(traffic_type) >= Self::MAX_QUEUE_SIZE * 2
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

    pub(crate) async fn write_raw(&self, data: &[u8]) -> anyhow::Result<()> {
        let mut written = 0;
        loop {
            self.stream.writable().await?;
            match self.stream.try_write(&data[written..]) {
                Ok(n) => {
                    written += n;
                    if written >= data.len() {
                        break;
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => {
                    bail!(e)
                }
            }
        }
        Ok(())
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        self.close_internal();
        let alive = LIVE_SOCKETS.fetch_sub(1, Ordering::Relaxed) - 1;
        debug!(socket_id = self.socket_id, alive, "Socket dropped");
    }
}

#[async_trait]
pub trait SocketExtensions {
    fn start(&self);
    fn async_write(
        &self,
        buffer: &Arc<Vec<u8>>,
        callback: Option<WriteCallback>,
        traffic_type: TrafficType,
    );
    fn close(&self);
    fn ongoing_checkup(&self);

    fn get_remote(&self) -> Option<SocketAddrV6>;
    fn has_timed_out(&self) -> bool;

    fn write_queued_messages(&self);
}

#[async_trait]
impl SocketExtensions for Arc<Socket> {
    fn start(&self) {
        self.ongoing_checkup();
    }

    fn async_write(
        &self,
        buffer: &Arc<Vec<u8>>,
        callback: Option<WriteCallback>,
        traffic_type: TrafficType,
    ) {
        if self.is_closed() {
            if let Some(cb) = callback {
                cb(ErrorCode::not_supported(), 0);
            }
            return;
        }

        let (queued, callback) = self
            .send_queue
            .insert(Arc::clone(buffer), callback, traffic_type);
        if !queued {
            if let Some(cb) = callback {
                cb(ErrorCode::not_supported(), 0);
            }
            return;
        }

        let self_clone = self.clone();
        let Some(runtime) = self.runtime.upgrade() else {
            return;
        };
        runtime.tokio.spawn_blocking(move || {
            if !self_clone.write_in_progress.load(Ordering::SeqCst) {
                self_clone.write_queued_messages();
            }
        });
    }

    fn write_queued_messages(&self) {
        if self.is_closed() {
            return;
        }

        let Some(mut next) = self.send_queue.pop() else {
            return;
        };
        self.set_default_timeout();
        self.write_in_progress.store(true, Ordering::SeqCst);
        let self_w = Arc::downgrade(self);

        let callback: Arc<Mutex<Option<Box<dyn FnOnce(ErrorCode, usize) + Send>>>> =
            Arc::new(Mutex::new(Some(Box::new(move |ec, size| {
                if let Some(self_clone) = self_w.upgrade() {
                    self_clone.write_in_progress.store(false, Ordering::SeqCst);

                    if ec.is_err() {
                        self_clone.observer.write_error();
                        self_clone.close();
                    } else {
                        self_clone.observer.write_successful(size);
                        self_clone.set_last_completion();
                    }

                    if let Some(cbk) = next.callback.take() {
                        cbk(ec, size);
                    }

                    if ec.is_ok() {
                        self_clone.write_queued_messages();
                    }
                }
            }))));

        let callback_clone = Arc::clone(&callback);
        {
            *self.current_action.lock().unwrap() = Some(Box::new(move || {
                let f = { callback_clone.lock().unwrap().take() };
                if let Some(f) = f {
                    f(ErrorCode::fault(), 0);
                }
            }));
        }

        let Some(runtime) = self.runtime.upgrade() else {
            return;
        };
        let runtime_w = Weak::clone(&self.runtime);
        let buffer = Arc::clone(&next.buffer);
        let stream = self.stream.clone();
        runtime.tokio.spawn(async move {
            let mut written = 0;
            loop {
                match stream.writable().await {
                    Ok(()) => match stream.try_write(&buffer[written..]) {
                        Ok(n) => {
                            written += n;
                            if written >= buffer.len() {
                                let Some(runtime) = runtime_w.upgrade() else {
                                    break;
                                };
                                runtime.tokio.spawn_blocking(move || {
                                    let f = { callback.lock().unwrap().take() };
                                    if let Some(cb) = f {
                                        cb(ErrorCode::new(), written);
                                    }
                                });
                                break;
                            }
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            continue;
                        }
                        Err(_) => {
                            let Some(runtime) = runtime_w.upgrade() else {
                                break;
                            };
                            runtime.tokio.spawn_blocking(move || {
                                let f = { callback.lock().unwrap().take() };
                                if let Some(cb) = f {
                                    cb(ErrorCode::fault(), 0);
                                }
                            });
                            break;
                        }
                    },
                    Err(_) => {
                        let Some(runtime) = runtime_w.upgrade() else {
                            break;
                        };
                        runtime.tokio.spawn_blocking(move || {
                            let f = { callback.lock().unwrap().take() };
                            if let Some(cb) = f {
                                cb(ErrorCode::fault(), 0);
                            }
                        });
                        break;
                    }
                }
            }
        });
    }

    fn close(&self) {
        self.close_internal();
    }

    fn ongoing_checkup(&self) {
        let socket = Arc::downgrade(self);
        self.thread_pool.add_delayed_task(
            Duration::from_secs(2),
            Box::new(move || {
                if let Some(socket) = socket.upgrade() {
                    // If the socket is already dead, close just in case, and stop doing checkups
                    if !socket.is_alive() {
                        socket.close();
                        return;
                    }

                    let now = seconds_since_epoch();
                    let mut condition_to_disconnect = false;

                    // if this is a server socket, and no data is received for silent_connection_tolerance_time seconds then disconnect
                    if socket.direction == ChannelDirection::Inbound
                        && (now - socket.last_receive_time_or_init.load(Ordering::SeqCst))
                            > socket
                                .silent_connection_tolerance_time
                                .load(Ordering::SeqCst)
                    {
                        socket.observer.silent_connection_dropped();
                        condition_to_disconnect = true;
                    }

                    // if there is no activity for timeout seconds then disconnect
                    if (now - socket.last_completion_time_or_init.load(Ordering::SeqCst))
                        > socket.timeout_seconds.load(Ordering::SeqCst)
                    {
                        socket
                            .observer
                            .inactive_connection_dropped(socket.direction);
                        condition_to_disconnect = true;
                    }

                    if condition_to_disconnect {
                        if let Some(ep) = socket.get_remote() {
                            socket.observer.disconnect_due_to_timeout(ep);
                        }
                        socket.timed_out.store(true, Ordering::SeqCst);
                        socket.close();
                    } else if !socket.is_closed() {
                        socket.ongoing_checkup();
                    }
                }
            }),
        );
    }

    fn get_remote(&self) -> Option<SocketAddrV6> {
        Some(self.remote)
    }

    fn has_timed_out(&self) -> bool {
        self.timed_out.load(Ordering::SeqCst)
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
    thread_pool: Arc<dyn ThreadPool>,
    default_timeout: Duration,
    silent_connection_tolerance_time: Duration,
    idle_timeout: Duration,
    observer: Option<Arc<dyn SocketObserver>>,
    max_write_queue_len: usize,
    async_runtime: Weak<AsyncRuntime>,
}

static NEXT_SOCKET_ID: AtomicUsize = AtomicUsize::new(0);
static LIVE_SOCKETS: AtomicUsize = AtomicUsize::new(0);

pub fn alive_sockets() -> usize {
    LIVE_SOCKETS.load(Ordering::Relaxed)
}

impl SocketBuilder {
    pub fn new(
        direction: ChannelDirection,
        thread_pool: Arc<dyn ThreadPool>,
        async_runtime: Weak<AsyncRuntime>,
    ) -> Self {
        Self {
            direction,
            thread_pool,
            default_timeout: Duration::from_secs(15),
            silent_connection_tolerance_time: Duration::from_secs(120),
            idle_timeout: Duration::from_secs(120),
            observer: None,
            max_write_queue_len: Socket::MAX_QUEUE_SIZE,
            async_runtime,
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

    pub fn finish(self, stream: TcpStream) -> Arc<Socket> {
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
        Arc::new({
            Socket {
                socket_id,
                remote,
                last_completion_time_or_init: AtomicU64::new(seconds_since_epoch()),
                last_receive_time_or_init: AtomicU64::new(seconds_since_epoch()),
                default_timeout: AtomicU64::new(self.default_timeout.as_secs()),
                timeout_seconds: AtomicU64::new(u64::MAX),
                idle_timeout: self.idle_timeout,
                thread_pool: self.thread_pool,
                direction: self.direction,
                silent_connection_tolerance_time: AtomicU64::new(
                    self.silent_connection_tolerance_time.as_secs(),
                ),
                timed_out: AtomicBool::new(false),
                closed: AtomicBool::new(false),
                socket_type: AtomicU8::new(ChannelMode::Undefined as u8),
                observer,
                write_in_progress: AtomicBool::new(false),
                send_queue: WriteQueue::new(self.max_write_queue_len),
                runtime: self.async_runtime,
                current_action: Mutex::new(None),
                stream: Arc::new(stream),
            }
        })
    }
}
