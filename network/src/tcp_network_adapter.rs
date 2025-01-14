use crate::{
    utils::into_ipv6_socket_address, ChannelDirection, ChannelId, ChannelMode,
    DeadChannelCleanupStep, Network, TcpChannelAdapter,
};
use rsnano_core::utils::NULL_ENDPOINT;
use rsnano_nullable_clock::SteadyClock;
use rsnano_nullable_tcp::TcpStream;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
    time::{Duration, Instant},
};
use tracing::{debug, warn};

/// Connects the Network to TcpStreams
pub struct TcpNetworkAdapter {
    channel_adapters: Mutex<HashMap<ChannelId, Arc<TcpChannelAdapter>>>,
    pub network: Arc<RwLock<Network>>,
    clock: Arc<SteadyClock>,
    handle: tokio::runtime::Handle,
}

impl TcpNetworkAdapter {
    pub fn new(
        network_info: Arc<RwLock<Network>>,
        clock: Arc<SteadyClock>,
        handle: tokio::runtime::Handle,
    ) -> Self {
        Self {
            channel_adapters: Mutex::new(HashMap::new()),
            clock,
            network: network_info,
            handle,
        }
    }

    pub async fn wait_for_available_inbound_slot(&self) {
        let last_log = Instant::now();
        let log_interval = Duration::from_secs(15);
        while self.should_wait_for_inbound_slot() {
            if last_log.elapsed() >= log_interval {
                warn!("Waiting for available slots to accept new connections");
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    fn should_wait_for_inbound_slot(&self) -> bool {
        let network = self.network.read().unwrap();
        !network.is_inbound_slot_available() && !network.is_stopped()
    }

    pub fn add(
        &self,
        stream: TcpStream,
        direction: ChannelDirection,
    ) -> anyhow::Result<Arc<TcpChannelAdapter>> {
        let peer_addr = stream
            .peer_addr()
            .map(into_ipv6_socket_address)
            .unwrap_or(NULL_ENDPOINT);

        let local_addr = stream
            .local_addr()
            .map(into_ipv6_socket_address)
            .unwrap_or(NULL_ENDPOINT);

        let channel =
            self.network
                .write()
                .unwrap()
                .add(local_addr, peer_addr, direction, self.clock.now());

        let channel = channel.map_err(|e| anyhow!("Could not add channel: {:?}", e))?;
        let channel_id = channel.channel_id();
        let channel_adapter =
            TcpChannelAdapter::create(channel, stream, self.clock.clone(), &self.handle);

        self.channel_adapters
            .lock()
            .unwrap()
            .insert(channel_id, channel_adapter.clone());

        debug!(?peer_addr, ?direction, "Accepted connection");

        Ok(channel_adapter)
    }

    pub fn set_listening_port(&self, port: u16) {
        self.network.write().unwrap().set_listening_port(port);
    }

    pub fn new_null(handle: tokio::runtime::Handle) -> Self {
        Self::new(
            Arc::new(RwLock::new(Network::new_test_instance())),
            Arc::new(SteadyClock::new_null()),
            handle,
        )
    }
}

pub struct NetworkCleanup(Arc<TcpNetworkAdapter>);

impl NetworkCleanup {
    pub fn new(network: Arc<TcpNetworkAdapter>) -> Self {
        Self(network)
    }
}

impl DeadChannelCleanupStep for NetworkCleanup {
    fn clean_up_dead_channels(&self, dead_channel_ids: &[ChannelId]) {
        let mut channels = self.0.channel_adapters.lock().unwrap();
        for channel_id in dead_channel_ids {
            channels.remove(channel_id);
        }
    }
}
