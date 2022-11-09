use crate::utils::{seconds_since_epoch, BufferWrapper, ErrorCode, ThreadPool};
use num_traits::FromPrimitive;
use std::{
    any::Any,
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicU8, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

/// Policy to affect at which stage a buffer can be dropped
#[derive(PartialEq, Eq, FromPrimitive)]
pub enum BufferDropPolicy {
    /// Can be dropped by bandwidth limiter (default)
    Limiter,
    /// Should not be dropped by bandwidth limiter
    NoLimiterDrop,
    /// Should not be dropped by bandwidth limiter or socket write queue limiter
    NoSocketDrop,
}

pub trait TcpSocketFacade: Send + Sync {
    fn local_endpoint(&self) -> SocketAddr;
    fn async_connect(&self, endpoint: SocketAddr, callback: Box<dyn FnOnce(ErrorCode)>);
    fn async_read(
        &self,
        buffer: &Arc<dyn BufferWrapper>,
        len: usize,
        callback: Box<dyn FnOnce(ErrorCode, usize)>,
    );
    fn async_read2(
        &self,
        buffer: &Arc<Mutex<Vec<u8>>>,
        len: usize,
        callback: Box<dyn FnOnce(ErrorCode, usize)>,
    );
    fn async_write(&self, buffer: &Arc<Vec<u8>>, callback: Box<dyn FnOnce(ErrorCode, usize)>);
    fn remote_endpoint(&self) -> Result<SocketAddr, ErrorCode>;
    fn post(&self, f: Box<dyn FnOnce()>);
    fn dispatch(&self, f: Box<dyn FnOnce()>);
    fn close(&self) -> Result<(), ErrorCode>;
    fn as_any(&self) -> &dyn Any;
}

#[derive(PartialEq, Eq, Clone, Copy, FromPrimitive)]
pub enum EndpointType {
    Server,
    Client,
}

#[derive(PartialEq, Eq, Clone, Copy, FromPrimitive)]
pub enum SocketType {
    Undefined,
    Bootstrap,
    Realtime,
    RealtimeResponseServer, // special type for tcp channel response server
}

impl SocketType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SocketType::Undefined => "undefined",
            SocketType::Bootstrap => "bootstrap",
            SocketType::Realtime => "realtime",
            SocketType::RealtimeResponseServer => "realtime_response_server",
        }
    }
}

pub trait SocketObserver: Send + Sync {
    fn close_socket_failed(&self, ec: ErrorCode);
    fn disconnect_due_to_timeout(&self, endpoint: SocketAddr);
    fn connect_error(&self);
    fn read_error(&self);
    fn read_successful(&self, len: usize);
    fn write_error(&self);
    fn write_successful(&self, len: usize);
    fn silent_connection_dropped(&self);
    fn inactive_connection_dropped(&self, endpoint_type: EndpointType);
}

#[derive(Default)]
pub struct NullSocketObserver {}

impl NullSocketObserver {
    pub fn new() -> Self {
        Default::default()
    }
}

impl SocketObserver for NullSocketObserver {
    fn close_socket_failed(&self, _ec: ErrorCode) {}
    fn disconnect_due_to_timeout(&self, _endpoint: SocketAddr) {}
    fn connect_error(&self) {}
    fn read_error(&self) {}
    fn read_successful(&self, _len: usize) {}
    fn write_error(&self) {}
    fn write_successful(&self, _len: usize) {}
    fn silent_connection_dropped(&self) {}
    fn inactive_connection_dropped(&self, _endpoint_type: EndpointType) {}
}

pub struct SocketImpl {
    /// The other end of the connection
    remote: Mutex<Option<SocketAddr>>,

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

    pub tcp_socket: Arc<dyn TcpSocketFacade>,
    thread_pool: Arc<dyn ThreadPool>,
    endpoint_type: EndpointType,
    /// used in real time server sockets, number of seconds of no receive traffic that will cause the socket to timeout
    pub silent_connection_tolerance_time: AtomicU64,

    /// Flag that is set when cleanup decides to close the socket due to timeout.
    /// NOTE: Currently used by tcp_server::timeout() but I suspect that this and tcp_server::timeout() are not needed.
    timed_out: AtomicBool,

    /// Set by close() - completion handlers must check this. This is more reliable than checking
    /// error codes as the OS may have already completed the async operation.
    closed: AtomicBool,

    /// Tracks number of blocks queued for delivery to the local socket send buffers.
    ///  Under normal circumstances, this should be zero.
    ///  Note that this is not the number of buffers queued to the peer, it is the number of buffers
    ///  queued up to enter the local TCP send buffer
    ///  socket buffer queue -> TCP send queue -> (network) -> TCP receive queue of peer
    queue_size: AtomicUsize,

    socket_type: AtomicU8,

    observer: Arc<dyn SocketObserver>,
}

impl SocketImpl {
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

    fn set_default_timeout(&self) {
        self.set_default_timeout_value(self.default_timeout.load(Ordering::SeqCst));
    }

    pub fn set_default_timeout_value(&self, seconds: u64) {
        self.timeout_seconds.store(seconds, Ordering::SeqCst);
    }

    pub fn close_internal(&self) {
        if !self.closed.swap(true, Ordering::SeqCst) {
            self.set_default_timeout_value(0);

            if let Err(ec) = self.tcp_socket.close() {
                self.observer.close_socket_failed(ec);
            }
        }
    }

    pub fn socket_type(&self) -> SocketType {
        SocketType::from_u8(self.socket_type.load(Ordering::SeqCst)).unwrap()
    }

    pub fn set_socket_type(&self, socket_type: SocketType) {
        self.socket_type.store(socket_type as u8, Ordering::SeqCst);
    }

    pub fn endpoint_type(&self) -> EndpointType {
        self.endpoint_type
    }

    pub fn local_endpoint(&self) -> SocketAddr {
        self.tcp_socket.local_endpoint()
    }

    pub fn is_realtime_connection(&self) -> bool {
        self.socket_type() == SocketType::Realtime
            || self.socket_type() == SocketType::RealtimeResponseServer
    }

    const QUEUE_SIZE_MAX: usize = 128;
    pub fn max(&self) -> bool {
        self.queue_size.load(Ordering::Relaxed) >= Self::QUEUE_SIZE_MAX
    }

    pub fn full(&self) -> bool {
        self.queue_size.load(Ordering::Relaxed) >= Self::QUEUE_SIZE_MAX * 2
    }

    pub fn is_bootstrap_connection(&self) -> bool {
        self.socket_type() == SocketType::Bootstrap
    }

    pub fn default_timeout_value(&self) -> u64 {
        self.default_timeout.load(Ordering::SeqCst)
    }
}

impl Drop for SocketImpl {
    fn drop(&mut self) {
        self.close_internal();
    }
}

pub trait Socket {
    fn async_connect(&self, endpoint: SocketAddr, callback: Box<dyn Fn(ErrorCode)>);
    fn async_read(
        &self,
        buffer: Arc<dyn BufferWrapper>,
        size: usize,
        callback: Box<dyn Fn(ErrorCode, usize)>,
    );
    fn async_read2(
        &self,
        buffer: Arc<Mutex<Vec<u8>>>,
        size: usize,
        callback: Box<dyn FnOnce(ErrorCode, usize)>,
    );
    fn async_write(
        &self,
        buffer: &Arc<Vec<u8>>,
        callback: Option<Box<dyn FnOnce(ErrorCode, usize)>>,
    );
    fn close(&self);
    fn checkup(&self);

    fn get_remote(&self) -> Option<SocketAddr>;
    fn set_remote(&self, endpoint: SocketAddr);
    fn has_timed_out(&self) -> bool;
    fn get_queue_size(&self) -> usize;
    fn set_silent_connection_tolerance_time(&self, time_s: u64);
}

impl Socket for Arc<SocketImpl> {
    fn async_connect(&self, endpoint: SocketAddr, callback: Box<dyn Fn(ErrorCode)>) {
        let self_clone = self.clone();
        debug_assert!(self.endpoint_type == EndpointType::Client);
        self.checkup();
        self.set_default_timeout();
        self.tcp_socket.async_connect(
            endpoint,
            Box::new(move |ec| {
                if !ec.is_err() {
                    self_clone.set_last_completion()
                }
                {
                    let mut lk = self_clone.remote.lock().unwrap();
                    *lk = Some(endpoint);
                }

                if ec.is_err() {
                    self_clone.observer.connect_error();
                }
                callback(ec);
            }),
        );
    }

    fn async_read(
        &self,
        buffer: Arc<dyn BufferWrapper>,
        size: usize,
        callback: Box<dyn Fn(ErrorCode, usize)>,
    ) {
        if size <= buffer.len() {
            if !self.is_closed() {
                self.set_default_timeout();
                let self_clone = self.clone();

                self.tcp_socket.async_read(
                    &buffer,
                    size,
                    Box::new(move |ec, len| {
                        if ec.is_err() {
                            self_clone.observer.read_error();
                        } else {
                            self_clone.observer.read_successful(len);
                            self_clone.set_last_completion();
                            self_clone.set_last_receive_time();
                        }
                        callback(ec, len);
                    }),
                );
            }
        } else {
            debug_assert!(false); // async_read called with incorrect buffer size
            callback(ErrorCode::no_buffer_space(), 0);
        }
    }

    fn async_read2(
        &self,
        buffer: Arc<Mutex<Vec<u8>>>,
        size: usize,
        callback: Box<dyn FnOnce(ErrorCode, usize)>,
    ) {
        let buffer_len = { buffer.lock().unwrap().len() };
        if size <= buffer_len {
            if !self.is_closed() {
                self.set_default_timeout();
                let self_clone = self.clone();

                self.tcp_socket.async_read2(
                    &buffer,
                    size,
                    Box::new(move |ec, len| {
                        if ec.is_err() {
                            self_clone.observer.read_error();
                        } else {
                            self_clone.observer.read_successful(len);
                            self_clone.set_last_completion();
                            self_clone.set_last_receive_time();
                        }
                        callback(ec, len);
                    }),
                );
            }
        } else {
            debug_assert!(false); // async_read called with incorrect buffer size
            callback(ErrorCode::no_buffer_space(), 0);
        }
    }

    fn async_write(
        &self,
        buffer: &Arc<Vec<u8>>,
        mut callback: Option<Box<dyn FnOnce(ErrorCode, usize)>>,
    ) {
        if self.is_closed() {
            if let Some(cb) = callback {
                self.tcp_socket.post(Box::new(move || {
                    cb(ErrorCode::not_supported(), 0);
                }));
            }

            return;
        }

        self.queue_size.fetch_add(1, Ordering::SeqCst);

        let self_clone = self.clone();
        let buffer = Arc::clone(buffer);
        self.tcp_socket.post(Box::new(move || {
            if self_clone.is_closed() {
                if let Some(cb) = callback.take() {
                    cb(ErrorCode::not_supported(), 0);
                }

                return;
            }

            self_clone.set_default_timeout();
            let self_clone_2 = self_clone.clone();

            self_clone.tcp_socket.async_write(
                &buffer,
                Box::new(move |ec, size| {
                    let _ = buffer;
                    self_clone_2.queue_size.fetch_sub(1, Ordering::SeqCst);

                    if ec.is_err() {
                        self_clone_2.observer.write_error();
                    } else {
                        self_clone_2.observer.write_successful(size);
                        self_clone_2.set_last_completion();
                    }

                    if let Some(cbk) = callback.take() {
                        cbk(ec, size);
                    }
                }),
            );
        }));
    }

    fn close(&self) {
        let clone = self.clone();
        self.tcp_socket.dispatch(Box::new(move || {
            clone.close_internal();
        }));
    }

    fn checkup(&self) {
        let socket = Arc::downgrade(self);
        self.thread_pool.add_timed_task(
            Duration::from_secs(2),
            Box::new(move || {
                if let Some(socket) = socket.upgrade() {
                    let now = seconds_since_epoch();
                    let mut condition_to_disconnect = false;

                    // if this is a server socket, and no data is received for silent_connection_tolerance_time seconds then disconnect
                    if socket.endpoint_type == EndpointType::Server
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
                            .inactive_connection_dropped(socket.endpoint_type);
                        condition_to_disconnect = true;
                    }

                    if condition_to_disconnect {
                        // The remote end may have closed the connection before this side timing out, in which case the remote address is no longer available.
                        if let Ok(ep) = socket.tcp_socket.remote_endpoint() {
                            socket.observer.disconnect_due_to_timeout(ep);
                        }
                        socket.timed_out.store(true, Ordering::SeqCst);
                        socket.close();
                    } else if !socket.is_closed() {
                        socket.checkup();
                    }
                }
            }),
        );
    }

    fn get_remote(&self) -> Option<SocketAddr> {
        *self.remote.lock().unwrap()
    }

    fn set_remote(&self, endpoint: SocketAddr) {
        let mut lk = self.remote.lock().unwrap();
        *lk = Some(endpoint);
    }

    fn has_timed_out(&self) -> bool {
        self.timed_out.load(Ordering::SeqCst)
    }

    fn get_queue_size(&self) -> usize {
        self.queue_size.load(Ordering::SeqCst)
    }

    fn set_silent_connection_tolerance_time(&self, time_s: u64) {
        let socket = Arc::clone(self);
        self.tcp_socket.dispatch(Box::new(move || {
            socket
                .silent_connection_tolerance_time
                .store(time_s, Ordering::SeqCst);
        }));
    }
}

pub struct SocketBuilder {
    endpoint_type: EndpointType,
    tcp_facade: Arc<dyn TcpSocketFacade>,
    thread_pool: Arc<dyn ThreadPool>,
    default_timeout: Duration,
    silent_connection_tolerance_time: Duration,
    observer: Option<Arc<dyn SocketObserver>>,
}

impl SocketBuilder {
    pub fn endpoint_type(
        endpoint_type: EndpointType,
        tcp_facade: Arc<dyn TcpSocketFacade>,
        thread_pool: Arc<dyn ThreadPool>,
    ) -> Self {
        Self {
            endpoint_type,
            tcp_facade,
            thread_pool,
            default_timeout: Duration::from_secs(15),
            silent_connection_tolerance_time: Duration::from_secs(120),
            observer: None,
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

    pub fn observer(mut self, observer: Arc<dyn SocketObserver>) -> Self {
        self.observer = Some(observer);
        self
    }

    pub fn build(self) -> Arc<SocketImpl> {
        let observer = self
            .observer
            .unwrap_or_else(|| Arc::new(NullSocketObserver::new()));
        Arc::new({
            SocketImpl {
                remote: Mutex::new(None),
                last_completion_time_or_init: AtomicU64::new(seconds_since_epoch()),
                last_receive_time_or_init: AtomicU64::new(seconds_since_epoch()),
                tcp_socket: self.tcp_facade,
                default_timeout: AtomicU64::new(self.default_timeout.as_secs()),
                timeout_seconds: AtomicU64::new(u64::MAX),
                thread_pool: self.thread_pool,
                endpoint_type: self.endpoint_type,
                silent_connection_tolerance_time: AtomicU64::new(
                    self.silent_connection_tolerance_time.as_secs(),
                ),
                timed_out: AtomicBool::new(false),
                closed: AtomicBool::new(false),
                queue_size: AtomicUsize::new(0),
                socket_type: AtomicU8::new(SocketType::Undefined as u8),
                observer,
            }
        })
    }
}
