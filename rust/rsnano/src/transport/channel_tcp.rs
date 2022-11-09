use std::{
    fmt::Display,
    net::{IpAddr, Ipv6Addr, SocketAddr},
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        Arc, Mutex, MutexGuard, Weak,
    },
};

use super::{
    BandwidthLimitType, BufferDropPolicy, Channel, OutboundBandwidthLimiter, Socket, SocketImpl,
};
use crate::{
    core::{messages::Message, Account},
    ffi::ChannelTcpObserverWeakPtr,
    utils::{ErrorCode, IoContext},
};

pub trait ChannelTcpObserver {
    fn data_sent(&self, endpoint: &SocketAddr);
    fn host_unreachable(&self);
    fn message_sent(&self, message: &dyn Message);
    fn message_dropped(&self, message: &dyn Message, buffer_size: usize);
    fn no_socket_drop(&self);
    fn write_drop(&self);
}

pub struct TcpChannelData {
    last_bootstrap_attempt: u64,
    last_packet_received: u64,
    last_packet_sent: u64,
    node_id: Option<Account>,
    pub endpoint: SocketAddr,
    pub peering_endpoint: Option<SocketAddr>,
}

pub struct ChannelTcp {
    channel_mutex: Mutex<TcpChannelData>,
    socket: Weak<SocketImpl>,
    /* Mark for temporary channels. Usually remote ports of these channels are ephemeral and received from incoming connections to server.
    If remote part has open listening port, temporary channel will be replaced with direct connection to listening port soon.
    But if other side is behing NAT or firewall this connection can be pemanent. */
    temporary: AtomicBool,
    network_version: AtomicU8,
    pub observer: ChannelTcpObserverWeakPtr,
    pub limiter: Arc<OutboundBandwidthLimiter>,
    pub io_ctx: Arc<dyn IoContext>,
}

impl ChannelTcp {
    pub fn new(
        socket: &Arc<SocketImpl>,
        now: u64,
        observer: ChannelTcpObserverWeakPtr,
        limiter: Arc<OutboundBandwidthLimiter>,
        io_ctx: Arc<dyn IoContext>,
    ) -> Self {
        Self {
            channel_mutex: Mutex::new(TcpChannelData {
                last_bootstrap_attempt: 0,
                last_packet_received: now,
                last_packet_sent: now,
                node_id: None,
                endpoint: SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0),
                peering_endpoint: None,
            }),
            socket: Arc::downgrade(socket),
            temporary: AtomicBool::new(false),
            network_version: AtomicU8::new(0),
            observer,
            limiter,
            io_ctx,
        }
    }

    pub fn socket(&self) -> Option<Arc<SocketImpl>> {
        self.socket.upgrade()
    }

    pub fn lock(&self) -> MutexGuard<TcpChannelData> {
        self.channel_mutex.lock().unwrap()
    }

    pub fn network_version(&self) -> u8 {
        self.network_version.load(Ordering::Relaxed)
    }

    pub fn set_network_version(&self, version: u8) {
        self.network_version.store(version, Ordering::Relaxed)
    }

    pub fn endpoint(&self) -> SocketAddr {
        self.channel_mutex.lock().unwrap().endpoint
    }

    pub fn set_endpoint(&self) {
        let mut lock = self.channel_mutex.lock().unwrap();
        debug_assert!(lock.endpoint == SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0)); // Not initialized endpoint value
                                                                                               // Calculate TCP socket endpoint
        if let Some(socket) = self.socket() {
            if let Some(ep) = socket.get_remote() {
                lock.endpoint = ep;
            }
        }
    }

    pub fn peering_endpoint(&self) -> SocketAddr {
        let lock = self.channel_mutex.lock().unwrap();
        match lock.peering_endpoint {
            Some(addr) => addr,
            None => lock.endpoint,
        }
    }

    pub fn set_peering_endpoint(&self, address: SocketAddr) {
        let mut lock = self.channel_mutex.lock().unwrap();
        lock.peering_endpoint = Some(address);
    }

    pub fn send_buffer(
        &self,
        buffer_a: &Arc<Vec<u8>>,
        callback_a: Option<Box<dyn FnOnce(ErrorCode, usize)>>,
        policy_a: BufferDropPolicy,
    ) {
        if let Some(socket_l) = self.socket() {
            if !socket_l.max() || (policy_a == BufferDropPolicy::NoSocketDrop && !socket_l.full()) {
                let observer_weak_l = self.observer.clone();
                let endpoint = socket_l
                    .get_remote()
                    .unwrap_or_else(|| SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0));
                socket_l.async_write(
                    buffer_a,
                    Some(Box::new(move |ec, size| {
                        if let Some(observer_l) = observer_weak_l.lock() {
                            if ec.is_ok() {
                                observer_l.data_sent(&endpoint);
                            }
                            if ec == ErrorCode::host_unreachable() {
                                observer_l.host_unreachable();
                            }
                        }
                        if let Some(callback) = callback_a {
                            callback(ec, size);
                        }
                    })),
                );
            } else {
                if let Some(observer_l) = self.observer.lock() {
                    if policy_a == BufferDropPolicy::NoSocketDrop {
                        observer_l.no_socket_drop();
                    } else {
                        observer_l.write_drop();
                    }
                }
                if let Some(callback_a) = callback_a {
                    callback_a(ErrorCode::no_buffer_space(), 0);
                }
            }
        } else if let Some(callback_a) = callback_a {
            self.io_ctx.post(Box::new(|| {
                callback_a(ErrorCode::not_supported(), 0);
            }));
        }
    }

    pub fn max(&self) -> bool {
        self.socket.upgrade().map(|s| s.max()).unwrap_or(true)
    }

    pub fn send(
        &self,
        message: &dyn Message,
        callback: Option<Box<dyn FnOnce(ErrorCode, usize)>>,
        drop_policy: BufferDropPolicy,
        limit_type: BandwidthLimitType,
    ) {
        let buffer = Arc::new(message.to_bytes());
        let is_droppable_by_limiter = drop_policy == BufferDropPolicy::Limiter;
        let should_pass = self.limiter.should_pass(buffer.len(), limit_type);
        if !is_droppable_by_limiter || should_pass {
            self.send_buffer(&buffer, callback, drop_policy);
            if let Some(observer) = self.observer.lock() {
                observer.message_sent(message);
            }
        } else {
            if let Some(callback) = callback {
                self.io_ctx.post(Box::new(move || {
                    callback(ErrorCode::not_supported(), 0);
                }));
            }

            if let Some(observer) = self.observer.lock() {
                observer.message_dropped(message, buffer.len());
            }
        }
    }
}

impl Display for ChannelTcp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.endpoint().fmt(f)
    }
}

impl Channel for ChannelTcp {
    fn is_temporary(&self) -> bool {
        self.temporary.load(Ordering::SeqCst)
    }

    fn set_temporary(&self, temporary: bool) {
        self.temporary.store(temporary, Ordering::SeqCst);
    }

    fn get_last_bootstrap_attempt(&self) -> u64 {
        self.channel_mutex.lock().unwrap().last_bootstrap_attempt
    }

    fn set_last_bootstrap_attempt(&self, instant: u64) {
        self.channel_mutex.lock().unwrap().last_bootstrap_attempt = instant;
    }

    fn get_last_packet_received(&self) -> u64 {
        self.channel_mutex.lock().unwrap().last_packet_received
    }

    fn set_last_packet_received(&self, instant: u64) {
        self.channel_mutex.lock().unwrap().last_packet_received = instant;
    }

    fn get_last_packet_sent(&self) -> u64 {
        self.channel_mutex.lock().unwrap().last_packet_sent
    }

    fn set_last_packet_sent(&self, instant: u64) {
        self.channel_mutex.lock().unwrap().last_packet_sent = instant;
    }

    fn get_node_id(&self) -> Option<Account> {
        self.channel_mutex.lock().unwrap().node_id
    }

    fn set_node_id(&self, id: Account) {
        self.channel_mutex.lock().unwrap().node_id = Some(id);
    }
}

impl Drop for ChannelTcp {
    fn drop(&mut self) {
        // Close socket. Exception: socket is used by bootstrap_server
        if let Some(socket) = self.socket.upgrade() {
            if !self.temporary.load(Ordering::Relaxed) {
                socket.close();
            }
        }
    }
}

impl PartialEq for ChannelTcp {
    fn eq(&self, other: &Self) -> bool {
        let my_socket = self.socket.upgrade();
        let other_socket = other.socket.upgrade();

        if my_socket.is_some() != other_socket.is_some() {
            return false;
        }

        if let Some(my_socket) = my_socket {
            if let Some(other_socket) = other_socket {
                if Arc::as_ptr(&my_socket) != Arc::as_ptr(&other_socket) {
                    return false;
                }
            }
        }

        true
    }
}
