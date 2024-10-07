use crate::{
    bandwidth_limiter::{OutboundBandwidthLimiter, OutboundBandwidthLimiterConfig},
    utils::into_ipv6_socket_address,
    Channel, ChannelDirection, ChannelId, ChannelMode, DeadChannelCleanupStep, DropPolicy,
    NetworkInfo, NetworkObserver, NullNetworkObserver, TrafficType,
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

pub struct Network {
    channels: Mutex<HashMap<ChannelId, Arc<Channel>>>,
    pub info: Arc<RwLock<NetworkInfo>>,
    limiter: Arc<OutboundBandwidthLimiter>,
    clock: Arc<SteadyClock>,
    observer: Arc<dyn NetworkObserver>,
    handle: tokio::runtime::Handle,
}

impl Network {
    pub fn new(
        limiter_config: OutboundBandwidthLimiterConfig,
        network_info: Arc<RwLock<NetworkInfo>>,
        clock: Arc<SteadyClock>,
        handle: tokio::runtime::Handle,
    ) -> Self {
        Self {
            channels: Mutex::new(HashMap::new()),
            limiter: Arc::new(OutboundBandwidthLimiter::new(limiter_config)),
            clock,
            info: network_info,
            observer: Arc::new(NullNetworkObserver::new()),
            handle,
        }
    }

    pub fn set_observer(&mut self, observer: Arc<dyn NetworkObserver>) {
        self.observer = observer;
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
    ) -> anyhow::Result<Arc<Channel>> {
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

        let channel_info = match channel_info {
            Ok(c) => {
                self.observer.accepted(&peer_addr, direction);
                c
            }
            Err(e) => {
                self.observer.error(e, &peer_addr, direction);
                return Err(anyhow!("Could not add channel: {:?}", e));
            }
        };

        let channel = Channel::create(
            channel_info,
            stream,
            self.limiter.clone(),
            self.clock.clone(),
            self.observer.clone(),
            &self.handle,
        );

        self.channels
            .lock()
            .unwrap()
            .insert(channel.channel_id(), channel.clone());

        debug!(?peer_addr, ?direction, "Accepted connection");

        Ok(channel)
    }

    pub fn new_null(handle: tokio::runtime::Handle) -> Self {
        Self::new(
            Default::default(),
            Arc::new(RwLock::new(NetworkInfo::new_test_instance())),
            Arc::new(SteadyClock::new_null()),
            handle,
        )
    }

    pub fn try_send_buffer(
        &self,
        channel_id: ChannelId,
        buffer: &[u8],
        drop_policy: DropPolicy,
        traffic_type: TrafficType,
    ) -> bool {
        let channel = self.channels.lock().unwrap().get(&channel_id).cloned();
        if let Some(channel) = channel {
            channel.try_send_buffer(buffer, drop_policy, traffic_type)
        } else {
            false
        }
    }

    pub async fn send_buffer(
        &self,
        channel_id: ChannelId,
        buffer: &[u8],
        traffic_type: TrafficType,
    ) -> anyhow::Result<()> {
        let channel = self.channels.lock().unwrap().get(&channel_id).cloned();
        if let Some(channel) = channel {
            channel.send_buffer(buffer, traffic_type).await
        } else {
            Err(anyhow!("Channel not found"))
        }
    }

    pub fn port(&self) -> u16 {
        self.info.read().unwrap().listening_port()
    }
}

pub struct NetworkCleanup(Arc<Network>);

impl NetworkCleanup {
    pub fn new(network: Arc<Network>) -> Self {
        Self(network)
    }
}

impl DeadChannelCleanupStep for NetworkCleanup {
    fn clean_up_dead_channels(&self, dead_channel_ids: &[ChannelId]) {
        let mut channels = self.0.channels.lock().unwrap();
        for channel_id in dead_channel_ids {
            channels.remove(channel_id);
        }
    }
}
