use super::{
    nano_data_receiver::NanoDataReceiver, HandshakeProcess, InboundMessageQueue, LatestKeepalives,
    SynCookies,
};
use crate::{stats::Stats, NetworkParams};
use rsnano_core::PrivateKey;
use rsnano_messages::*;
use rsnano_network::{Channel, Network, TcpChannelAdapter};
use std::{
    sync::{Arc, Mutex, RwLock},
    time::Duration,
};
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

pub struct ResponseServer {
    channel_adapter: Arc<TcpChannelAdapter>,
    channel: Arc<Channel>,
    network_params: Arc<NetworkParams>,
    stats: Arc<Stats>,
    network: Arc<RwLock<Network>>,
    inbound_queue: Arc<InboundMessageQueue>,
    network_filter: Arc<NetworkFilter>,
    syn_cookies: Arc<SynCookies>,
    node_id: PrivateKey,
    latest_keepalives: Arc<Mutex<LatestKeepalives>>,
}

impl ResponseServer {
    pub fn new(
        network: Arc<RwLock<Network>>,
        inbound_queue: Arc<InboundMessageQueue>,
        channel_adapter: Arc<TcpChannelAdapter>,
        network_filter: Arc<NetworkFilter>,
        network_params: Arc<NetworkParams>,
        stats: Arc<Stats>,
        syn_cookies: Arc<SynCookies>,
        node_id: PrivateKey,
        latest_keepalives: Arc<Mutex<LatestKeepalives>>,
    ) -> Self {
        let channel = channel_adapter.channel.clone();
        Self {
            network,
            inbound_queue,
            channel,
            channel_adapter,
            syn_cookies: syn_cookies.clone(),
            node_id: node_id.clone(),
            network_params,
            stats: stats.clone(),
            network_filter,
            latest_keepalives,
        }
    }

    fn create_data_receiver(&self) -> Box<dyn DataReceiver + Send> {
        let handshake_process = HandshakeProcess::new(
            self.network_params.ledger.genesis_block.hash(),
            self.node_id.clone(),
            self.syn_cookies.clone(),
            self.stats.clone(),
            self.network_params.network.protocol_info(),
        );

        let message_deserializer = MessageDeserializer::new(
            self.network_params.network.protocol_info(),
            self.network_filter.clone(),
            self.network_params.network.work.clone(),
        );

        Box::new(NanoDataReceiver::new(
            self.channel.clone(),
            self.network_params.clone(),
            handshake_process,
            message_deserializer,
            self.inbound_queue.clone(),
            self.latest_keepalives.clone(),
            self.stats.clone(),
            self.network.clone(),
        ))
    }

    pub async fn run(&self) {
        let mut receiver = self.create_data_receiver();
        receiver.initialize();

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

impl Drop for ResponseServer {
    fn drop(&mut self) {
        debug!("Exiting server: {}", self.channel.peer_addr());
        self.channel.close();
    }
}

pub trait DataReceiver {
    fn initialize(&mut self);
    fn receive(&mut self, data: &[u8]) -> bool;
}
