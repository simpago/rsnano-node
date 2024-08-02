use super::{bootstrap_limits, BootstrapConnections};
use crate::transport::{ChannelEnum, ChannelTcp};
use std::{
    net::SocketAddrV6,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex, Weak,
    },
    time::{Duration, Instant},
};

pub struct BootstrapClient {
    observer: Weak<BootstrapConnections>,
    channel: Arc<ChannelEnum>,
    block_count: AtomicU64,
    block_rate: AtomicU64,
    pending_stop: AtomicBool,
    hard_stop: AtomicBool,
    start_time: Mutex<Instant>,
}

impl BootstrapClient {
    pub fn new(observer: &Arc<BootstrapConnections>, channel: Arc<ChannelEnum>) -> Self {
        if let ChannelEnum::Tcp(tcp) = channel.as_ref() {
            tcp.update_remote_endpoint();
        }
        Self {
            observer: Arc::downgrade(observer),
            channel,
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

    pub fn get_channel(&self) -> &Arc<ChannelEnum> {
        &self.channel
    }

    fn tcp_channel(&self) -> &Arc<ChannelTcp> {
        match self.channel.as_ref() {
            ChannelEnum::Tcp(tcp) => tcp,
            _ => panic!("not a tcp channel!"),
        }
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

    pub fn close(&self) {
        self.channel.close();
    }

    pub fn set_timeout(&self, timeout: Duration) {
        self.channel.set_timeout(timeout);
    }

    pub fn remote_addr(&self) -> SocketAddrV6 {
        self.channel.remote_addr()
    }

    pub fn channel_string(&self) -> String {
        self.tcp_channel().to_string()
    }
}

impl Drop for BootstrapClient {
    fn drop(&mut self) {
        if let Some(observer) = self.observer.upgrade() {
            observer.bootstrap_client_closed();
        }
    }
}
