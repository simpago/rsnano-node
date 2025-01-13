use crate::{
    utils::into_ipv6_socket_address, ChannelAdapter, ChannelDirection, ChannelId, ChannelMode,
    DeadChannelCleanupStep, Network,
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

/// Connects the Network to real TcpStreams
pub struct NetworkAdapter {
    channel_adapters: Mutex<HashMap<ChannelId, Arc<ChannelAdapter>>>,
    pub info: Arc<RwLock<Network>>,
    clock: Arc<SteadyClock>,
    handle: tokio::runtime::Handle,
}

impl NetworkAdapter {
    pub fn new(
        network_info: Arc<RwLock<Network>>,
        clock: Arc<SteadyClock>,
        handle: tokio::runtime::Handle,
    ) -> Self {
        Self {
            channel_adapters: Mutex::new(HashMap::new()),
            clock,
            info: network_info,
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
        let info = self.info.read().unwrap();
        !info.is_inbound_slot_available() && !info.is_stopped()
    }

    pub fn add(
        &self,
        stream: TcpStream,
        direction: ChannelDirection,
        planned_mode: ChannelMode,
    ) -> anyhow::Result<Arc<ChannelAdapter>> {
        let peer_addr = stream
            .peer_addr()
            .map(into_ipv6_socket_address)
            .unwrap_or(NULL_ENDPOINT);

        let local_addr = stream
            .local_addr()
            .map(into_ipv6_socket_address)
            .unwrap_or(NULL_ENDPOINT);

        let channel_info = self.info.write().unwrap().add(
            local_addr,
            peer_addr,
            direction,
            planned_mode,
            self.clock.now(),
        );

        let channel_info = channel_info.map_err(|e| anyhow!("Could not add channel: {:?}", e))?;
        let channel_adapter =
            ChannelAdapter::create(channel_info, stream, self.clock.clone(), &self.handle);

        self.channel_adapters
            .lock()
            .unwrap()
            .insert(channel_adapter.channel_id(), channel_adapter.clone());

        debug!(?peer_addr, ?direction, "Accepted connection");

        Ok(channel_adapter)
    }

    pub fn new_null(handle: tokio::runtime::Handle) -> Self {
        Self::new(
            Arc::new(RwLock::new(Network::new_test_instance())),
            Arc::new(SteadyClock::new_null()),
            handle,
        )
    }
}

pub struct NetworkCleanup(Arc<NetworkAdapter>);

impl NetworkCleanup {
    pub fn new(network: Arc<NetworkAdapter>) -> Self {
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
