use std::{
    net::{IpAddr, Ipv6Addr, SocketAddr},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

use crate::{
    core::messages::Message,
    transport::{BandwidthLimitType, BufferDropPolicy, ChannelTcp, Socket, SocketImpl},
    utils::ErrorCode,
};

use super::bootstrap_limits;

pub trait BootstrapClientObserver {
    fn bootstrap_client_closed(&self);
    fn to_weak(&self) -> Box<dyn BootstrapClientObserverWeakPtr>;
}

pub trait BootstrapClientObserverWeakPtr {
    fn upgrade(&self) -> Option<Arc<dyn BootstrapClientObserver>>;
}

pub struct BootstrapClient {
    observer: Box<dyn BootstrapClientObserverWeakPtr>,
    channel: Arc<ChannelTcp>,
    socket: Arc<SocketImpl>,
    receive_buffer: Arc<Mutex<Vec<u8>>>,
    block_count: AtomicU64,
    block_rate: AtomicU64,
    pending_stop: AtomicBool,
    hard_stop: AtomicBool,
    start_time: Mutex<Instant>,
}

impl BootstrapClient {
    pub fn new(
        observer: Arc<dyn BootstrapClientObserver>,
        channel: Arc<ChannelTcp>,
        socket: Arc<SocketImpl>,
    ) -> Self {
        channel.set_endpoint();
        Self {
            observer: observer.to_weak(),
            channel,
            socket,
            receive_buffer: Arc::new(Mutex::new(vec![0; 256])),
            block_count: AtomicU64::new(0),
            block_rate: AtomicU64::new(0f64.to_bits()),
            pending_stop: AtomicBool::new(false),
            hard_stop: AtomicBool::new(false),
            start_time: Mutex::new(Instant::now()),
        }
    }

    pub fn sample_block_rate(&self) -> f64 {
        let elapsed = {
            let elapsed_seconds = self.elapsed().as_secs_f64();
            if elapsed_seconds > bootstrap_limits::BOOTSTRAP_MINIMUM_ELAPSED_SECONDS_BLOCKRATE {
                elapsed_seconds
            } else {
                bootstrap_limits::BOOTSTRAP_MINIMUM_ELAPSED_SECONDS_BLOCKRATE
            }
        };
        let new_block_rate = self.block_count.load(Ordering::SeqCst) as f64 / elapsed;
        self.block_rate
            .store((new_block_rate).to_bits(), Ordering::SeqCst);
        new_block_rate
    }

    pub fn elapsed(&self) -> Duration {
        self.start_time.lock().unwrap().elapsed()
    }

    pub fn set_start_time(&self) {
        let mut lock = self.start_time.lock().unwrap();
        *lock = Instant::now();
    }

    pub fn get_channel(&self) -> &Arc<ChannelTcp> {
        &self.channel
    }

    pub fn get_socket(&self) -> &Arc<SocketImpl> {
        &self.socket
    }

    pub fn read_async(&self, size: usize, callback: Box<dyn FnOnce(ErrorCode, usize)>) {
        self.socket
            .async_read2(Arc::clone(&self.receive_buffer), size, callback);
    }

    pub fn receive_buffer(&self) -> Vec<u8> {
        self.receive_buffer.lock().unwrap().clone()
    }

    pub fn receive_buffer_len(&self) -> usize {
        self.receive_buffer.lock().unwrap().len()
    }

    pub fn send_buffer(
        &self,
        buffer_a: &Arc<Vec<u8>>,
        callback_a: Option<Box<dyn FnOnce(ErrorCode, usize)>>,
        policy_a: BufferDropPolicy,
    ) {
        self.channel.send_buffer(buffer_a, callback_a, policy_a);
    }

    pub fn send(
        &self,
        message: &dyn Message,
        callback: Option<Box<dyn FnOnce(ErrorCode, usize)>>,
        drop_policy: BufferDropPolicy,
        limit_type: BandwidthLimitType,
    ) {
        self.channel
            .send(message, callback, drop_policy, limit_type);
    }

    pub fn inc_block_count(&self) -> u64 {
        self.block_count.fetch_add(1, Ordering::SeqCst)
    }

    pub fn block_count(&self) -> u64 {
        self.block_count.load(Ordering::SeqCst)
    }

    pub fn block_rate(&self) -> f64 {
        f64::from_bits(self.block_rate.load(Ordering::SeqCst))
    }

    pub fn pending_stop(&self) -> bool {
        self.pending_stop.load(Ordering::SeqCst)
    }

    pub fn hard_stop(&self) -> bool {
        self.hard_stop.load(Ordering::SeqCst)
    }

    pub fn stop(&self, force: bool) {
        self.pending_stop.store(true, Ordering::SeqCst);
        if force {
            self.hard_stop.store(true, Ordering::SeqCst);
        }
    }

    pub fn close_socket(&self) {
        self.socket.close();
    }

    pub fn set_timeout(&self, timeout: Duration) {
        self.socket.set_timeout(timeout);
    }

    pub fn remote_endpoint(&self) -> SocketAddr {
        self.socket
            .get_remote()
            .unwrap_or_else(|| SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0))
    }

    pub fn channel_string(&self) -> String {
        self.channel.to_string()
    }

    pub fn tcp_endpoint(&self) -> SocketAddr {
        self.channel.endpoint()
    }
}

impl Drop for BootstrapClient {
    fn drop(&mut self) {
        if let Some(observer) = self.observer.upgrade() {
            observer.bootstrap_client_closed();
        }
    }
}
