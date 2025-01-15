use rsnano_network::{Channel, TcpChannelAdapter};
use std::{sync::Arc, time::Duration};
use tracing::debug;

#[derive(Clone, Debug, PartialEq)]
pub struct TcpConfig {
    pub max_inbound_connections: usize,
    pub max_outbound_connections: usize,
    pub max_attempts: usize,
    pub max_attempts_per_ip: usize,
    pub connect_timeout: Duration,
}

impl TcpConfig {
    pub fn for_dev_network() -> Self {
        Self {
            max_inbound_connections: 128,
            max_outbound_connections: 128,
            max_attempts: 128,
            max_attempts_per_ip: 128,
            connect_timeout: Duration::from_secs(5),
        }
    }
}

impl Default for TcpConfig {
    fn default() -> Self {
        Self {
            max_inbound_connections: 2048,
            max_outbound_connections: 2048,
            max_attempts: 60,
            max_attempts_per_ip: 1,
            connect_timeout: Duration::from_secs(60),
        }
    }
}

pub(crate) struct ResponseServer {
    channel_adapter: Arc<TcpChannelAdapter>,
    channel: Arc<Channel>,
}

impl ResponseServer {
    pub fn new(channel_adapter: Arc<TcpChannelAdapter>) -> Self {
        let channel = channel_adapter.channel.clone();
        Self {
            channel,
            channel_adapter,
        }
    }

    pub async fn run(&self, mut receiver: Box<dyn DataReceiver + Send>) {
        let mut buffer = [0u8; 1024];

        loop {
            // TODO: abort readable waiting if channel closed

            if let Err(e) = self.channel_adapter.readable().await {
                debug!(
                    "Error reading buffer: {:?} ({})",
                    e,
                    self.channel.peer_addr()
                );
                self.channel.close();
                return;
            }

            let read_count = match self.channel_adapter.try_read(&mut buffer) {
                Ok(n) => n,
                Err(e) => {
                    debug!(
                        "Error reading buffer: {:?} ({})",
                        e,
                        self.channel.peer_addr()
                    );
                    self.channel.close();
                    return;
                }
            };

            let new_data = &buffer[..read_count];

            if !receiver.receive(new_data) {
                break;
            }
        }
    }
}

pub trait DataReceiver {
    fn initialize(&mut self);
    fn receive(&mut self, data: &[u8]) -> bool;
}
