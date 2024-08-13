use super::{
    attempt_container::AttemptContainer, channel_container::ChannelContainer, BufferDropPolicy,
    Channel, ChannelDirection, ChannelId, ChannelMode, NetworkFilter, OutboundBandwidthLimiter,
    PeerExclusion, TcpConfig, TcpStream, TrafficType,
};
use crate::{
    config::{NetworkConstants, NodeFlags},
    stats::{DetailType, Direction, StatType, Stats},
    utils::{
        into_ipv6_socket_address, ipv4_address_or_ipv6_subnet, is_ipv4_mapped,
        map_address_to_subnetwork, reserved_address, SteadyClock, Timestamp,
    },
    NetworkParams, DEV_NETWORK_PARAMS,
};
use rand::{seq::SliceRandom, thread_rng};
use rsnano_core::{
    utils::{ContainerInfo, ContainerInfoComponent, NULL_ENDPOINT},
    Account, PublicKey,
};
use rsnano_messages::*;
use std::{
    net::{Ipv6Addr, SocketAddrV6},
    sync::{
        atomic::{AtomicBool, AtomicU16, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant, SystemTime},
};
use tracing::{debug, warn};

pub struct NetworkOptions {
    pub allow_local_peers: bool,
    pub tcp_config: TcpConfig,
    pub publish_filter: Arc<NetworkFilter>,
    pub network_params: NetworkParams,
    pub stats: Arc<Stats>,
    pub port: u16,
    pub flags: NodeFlags,
    pub limiter: Arc<OutboundBandwidthLimiter>,
    pub clock: Arc<SteadyClock>,
}

impl NetworkOptions {
    pub fn new_test_instance() -> Self {
        NetworkOptions {
            allow_local_peers: true,
            tcp_config: TcpConfig::for_dev_network(),
            publish_filter: Arc::new(NetworkFilter::default()),
            network_params: DEV_NETWORK_PARAMS.clone(),
            stats: Arc::new(Default::default()),
            port: 8088,
            flags: NodeFlags::default(),
            limiter: Arc::new(OutboundBandwidthLimiter::default()),
            clock: Arc::new(SteadyClock::new_null()),
        }
    }
}

pub struct Network {
    state: Mutex<State>,
    port: AtomicU16,
    stopped: AtomicBool,
    allow_local_peers: bool,
    flags: NodeFlags,
    stats: Arc<Stats>,
    next_channel_id: AtomicUsize,
    network_params: Arc<NetworkParams>,
    limiter: Arc<OutboundBandwidthLimiter>,
    tcp_config: TcpConfig,
    pub publish_filter: Arc<NetworkFilter>,
    clock: Arc<SteadyClock>,
}

impl Drop for Network {
    fn drop(&mut self) {
        self.stop();
    }
}

impl Network {
    pub fn new(options: NetworkOptions) -> Self {
        let network = Arc::new(options.network_params);

        Self {
            port: AtomicU16::new(options.port),
            stopped: AtomicBool::new(false),
            allow_local_peers: options.allow_local_peers,
            state: Mutex::new(State {
                attempts: Default::default(),
                channels: Default::default(),
                network_constants: network.network.clone(),
                new_realtime_channel_observers: Vec::new(),
                excluded_peers: PeerExclusion::new(),
                stats: options.stats.clone(),
                node_flags: options.flags.clone(),
                config: options.tcp_config.clone(),
            }),
            tcp_config: options.tcp_config,
            flags: options.flags,
            stats: options.stats,
            next_channel_id: AtomicUsize::new(1),
            network_params: network,
            limiter: options.limiter,
            publish_filter: options.publish_filter,
            clock: options.clock,
        }
    }

    pub(crate) fn channels_info(&self) -> ChannelsInfo {
        self.state.lock().unwrap().channels_info()
    }

    pub(crate) async fn wait_for_available_inbound_slot(&self) {
        let last_log = Instant::now();
        let log_interval = if self.network_params.network.is_dev_network() {
            Duration::from_secs(1)
        } else {
            Duration::from_secs(15)
        };
        while self.count_by_direction(ChannelDirection::Inbound)
            >= self.tcp_config.max_inbound_connections
            && !self.stopped.load(Ordering::SeqCst)
        {
            if last_log.elapsed() >= log_interval {
                warn!(
                    "Waiting for available slots to accept new connections (current: {} / max: {})",
                    self.count_by_direction(ChannelDirection::Inbound),
                    self.tcp_config.max_inbound_connections
                );
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    pub fn can_add_connection(
        &self,
        peer_addr: &SocketAddrV6,
        direction: ChannelDirection,
        mode: ChannelMode,
    ) -> AcceptResult {
        if direction == ChannelDirection::Outbound {
            if self.can_add_outbound_connection(&peer_addr, mode) {
                AcceptResult::Accepted
            } else {
                AcceptResult::Rejected
            }
        } else {
            self.check_limits(&peer_addr, direction)
        }
    }

    pub async fn add(
        &self,
        stream: TcpStream,
        direction: ChannelDirection,
        mode: ChannelMode,
    ) -> anyhow::Result<Arc<Channel>> {
        let peer_addr = stream
            .peer_addr()
            .map(into_ipv6_socket_address)
            .unwrap_or(NULL_ENDPOINT);

        let result = self.can_add_connection(&peer_addr, direction, mode);
        if result != AcceptResult::Accepted {
            self.stats.inc_dir(
                StatType::TcpListener,
                DetailType::AcceptRejected,
                direction.into(),
            );
            if direction == ChannelDirection::Outbound {
                self.stats.inc_dir(
                    StatType::TcpListener,
                    DetailType::ConnectFailure,
                    Direction::Out,
                );
            }
            debug!(?peer_addr, ?direction, ?mode, "Rejected connection");
            if direction == ChannelDirection::Inbound {
                self.stats.inc_dir(
                    StatType::TcpListener,
                    DetailType::AcceptFailure,
                    Direction::In,
                );
                // Refusal reason should be logged earlier
            }
            return Err(anyhow!("check_limits failed"));
        }

        self.stats.inc_dir(
            StatType::TcpListener,
            DetailType::AcceptSuccess,
            direction.into(),
        );

        if direction == ChannelDirection::Outbound {
            self.stats.inc_dir(
                StatType::TcpListener,
                DetailType::ConnectSuccess,
                Direction::Out,
            );
        }

        let channel = Channel::create(
            self.get_next_channel_id(),
            stream,
            direction,
            self.network_params.network.protocol_info(),
            self.stats.clone(),
            self.limiter.clone(),
        )
        .await;
        self.state.lock().unwrap().channels.insert(channel.clone());

        debug!(?peer_addr, ?direction, ?mode, "Accepted connection");

        Ok(channel)
    }

    pub(crate) fn new_null() -> Self {
        Self::new(NetworkOptions::new_test_instance())
    }

    pub(crate) fn stop(&self) {
        if !self.stopped.swap(true, Ordering::SeqCst) {
            self.close();
        }
    }

    fn close(&self) {
        self.state.lock().unwrap().close_channels();
    }

    pub fn get_next_channel_id(&self) -> ChannelId {
        self.next_channel_id.fetch_add(1, Ordering::SeqCst).into()
    }

    pub fn endpoint_for(&self, channel_id: ChannelId) -> Option<SocketAddrV6> {
        self.state
            .lock()
            .unwrap()
            .channels
            .get_by_id(channel_id)
            .map(|e| e.remote_addr())
    }

    pub fn not_a_peer(&self, endpoint: &SocketAddrV6, allow_local_peers: bool) -> bool {
        endpoint.ip().is_unspecified()
            || reserved_address(endpoint, allow_local_peers)
            || endpoint
                == &SocketAddrV6::new(Ipv6Addr::LOCALHOST, self.port.load(Ordering::SeqCst), 0, 0)
    }

    pub(crate) fn on_new_realtime_channel(
        &self,
        callback: Arc<dyn Fn(Arc<Channel>) + Send + Sync>,
    ) {
        self.state
            .lock()
            .unwrap()
            .new_realtime_channel_observers
            .push(callback);
    }

    pub(crate) fn check_limits(
        &self,
        ip: &SocketAddrV6,
        direction: ChannelDirection,
    ) -> AcceptResult {
        self.state
            .lock()
            .unwrap()
            .check_limits(ip, direction, self.clock.now())
    }

    pub(crate) fn add_attempt(&self, remote: SocketAddrV6) -> bool {
        let mut guard = self.state.lock().unwrap();

        let count = guard.attempts.count_by_address(remote.ip());
        if count >= self.tcp_config.max_attempts_per_ip {
            self.stats.inc_dir(
                StatType::TcpListenerRejected,
                DetailType::MaxAttemptsPerIp,
                Direction::Out,
            );
            debug!("Connection attempt already in progress ({}), unable to initiate new connection: {}", count, remote.ip());
            return false; // Rejected
        }

        guard.attempts.insert(remote, ChannelDirection::Outbound)
    }

    pub(crate) fn remove_attempt(&self, remote: &SocketAddrV6) {
        self.state.lock().unwrap().attempts.remove(&remote);
    }

    pub fn find_channels_by_remote_addr(&self, endpoint: &SocketAddrV6) -> Vec<Arc<Channel>> {
        self.state
            .lock()
            .unwrap()
            .find_channels_by_remote_addr(endpoint)
    }

    pub fn find_realtime_channel_by_remote_addr(
        &self,
        endpoint: &SocketAddrV6,
    ) -> Option<Arc<Channel>> {
        self.state
            .lock()
            .unwrap()
            .find_realtime_channel_by_remote_addr(endpoint)
    }

    pub(crate) fn find_channels_by_peering_addr(
        &self,
        peering_addr: &SocketAddrV6,
    ) -> Vec<Arc<Channel>> {
        self.state
            .lock()
            .unwrap()
            .find_channels_by_peering_addr(peering_addr)
    }

    pub(crate) fn find_realtime_channel_by_peering_addr(
        &self,
        peering_addr: &SocketAddrV6,
    ) -> Option<Arc<Channel>> {
        self.state
            .lock()
            .unwrap()
            .find_realtime_channel_by_peering_addr(peering_addr)
    }

    pub fn random_realtime_channels(&self, count: usize, min_version: u8) -> Vec<Arc<Channel>> {
        self.state
            .lock()
            .unwrap()
            .random_realtime_channels(count, min_version)
    }

    pub fn find_node_id(&self, node_id: &PublicKey) -> Option<Arc<Channel>> {
        self.state.lock().unwrap().find_node_id(node_id)
    }

    pub(crate) fn collect_container_info(&self, name: impl Into<String>) -> ContainerInfoComponent {
        self.state.lock().unwrap().collect_container_info(name)
    }

    pub fn random_fill_realtime(&self, endpoints: &mut [SocketAddrV6]) {
        self.state.lock().unwrap().random_fill_realtime(endpoints);
    }

    pub fn random_fanout_realtime(&self, scale: f32) -> Vec<Arc<Channel>> {
        self.state.lock().unwrap().random_fanout_realtime(scale)
    }

    pub(crate) fn random_list_realtime(&self, count: usize, min_version: u8) -> Vec<Arc<Channel>> {
        self.state
            .lock()
            .unwrap()
            .random_realtime_channels(count, min_version)
    }

    pub(crate) fn max(&self, channel_id: ChannelId, traffic_type: TrafficType) -> bool {
        self.state
            .lock()
            .unwrap()
            .channels
            .get_by_id(channel_id)
            .map(|c| c.max(traffic_type))
            .unwrap_or(true)
    }

    pub(crate) fn try_send(
        &self,
        channel_id: ChannelId,
        message: &Message,
        drop_policy: BufferDropPolicy,
        traffic_type: TrafficType,
    ) {
        if let Some(channel) = self.state.lock().unwrap().channels.get_by_id(channel_id) {
            channel.try_send(message, drop_policy, traffic_type);
        }
    }

    pub(crate) fn flood_message2(
        &self,
        message: &Message,
        drop_policy: BufferDropPolicy,
        scale: f32,
    ) {
        let channels = self.random_fanout_realtime(scale);
        for channel in channels {
            channel.try_send(message, drop_policy, TrafficType::Generic)
        }
    }

    pub fn flood_message(&self, message: &Message, scale: f32) {
        let channels = self.random_fanout_realtime(scale);
        for channel in channels {
            channel.try_send(message, BufferDropPolicy::Limiter, TrafficType::Generic)
        }
    }

    fn max_ip_or_subnetwork_connections(&self, endpoint: &SocketAddrV6) -> bool {
        self.max_ip_connections(endpoint) || self.max_subnetwork_connections(endpoint)
    }

    fn max_ip_connections(&self, endpoint: &SocketAddrV6) -> bool {
        if self.flags.disable_max_peers_per_ip {
            return false;
        }
        let mut result;
        let address = ipv4_address_or_ipv6_subnet(endpoint.ip());
        let lock = self.state.lock().unwrap();
        result = lock.channels.count_by_ip(&address) >= lock.network_constants.max_peers_per_ip;
        if !result {
            result =
                lock.attempts.count_by_address(&address) >= lock.network_constants.max_peers_per_ip;
        }
        if result {
            self.stats
                .inc_dir(StatType::Tcp, DetailType::MaxPerIp, Direction::Out);
        }
        result
    }

    fn max_subnetwork_connections(&self, endoint: &SocketAddrV6) -> bool {
        if self.flags.disable_max_peers_per_subnetwork {
            return false;
        }

        let subnet = map_address_to_subnetwork(endoint.ip());
        let is_max = {
            let guard = self.state.lock().unwrap();
            guard.channels.count_by_subnet(&subnet)
                >= self.network_params.network.max_peers_per_subnetwork
                || guard.attempts.count_by_subnetwork(&subnet)
                    >= self.network_params.network.max_peers_per_subnetwork
        };

        if is_max {
            self.stats
                .inc_dir(StatType::Tcp, DetailType::MaxPerSubnetwork, Direction::Out);
        }

        is_max
    }

    fn can_add_outbound_connection(&self, peer: &SocketAddrV6, mode: ChannelMode) -> bool {
        if self.flags.disable_tcp_realtime {
            return false;
        }

        // Don't contact invalid IPs
        if self.not_a_peer(peer, self.allow_local_peers) {
            return false;
        }

        // Don't overload single IP
        if self.max_ip_or_subnetwork_connections(peer) {
            return false;
        }

        let mut state = self.state.lock().unwrap();
        if state.excluded_peers.is_excluded(peer, self.clock.now()) {
            return false;
        }

        // Don't connect to nodes that already sent us something
        if state
            .find_channels_by_remote_addr(peer)
            .iter()
            .any(|c| c.mode() == mode || c.mode() == ChannelMode::Undefined)
        {
            return false;
        }
        if state
            .find_channels_by_peering_addr(peer)
            .iter()
            .any(|c| c.mode() == mode || c.mode() == ChannelMode::Undefined)
        {
            return false;
        }

        if state.check_limits(peer, ChannelDirection::Outbound, self.clock.now())
            != AcceptResult::Accepted
        {
            self.stats.inc_dir(
                StatType::TcpListener,
                DetailType::ConnectRejected,
                Direction::Out,
            );
            // Refusal reason should be logged earlier

            return false; // Rejected
        }

        self.stats.inc_dir(
            StatType::TcpListener,
            DetailType::ConnectInitiate,
            Direction::Out,
        );
        debug!("Initiate outgoing connection to: {}", peer);

        true
    }

    pub fn len_sqrt(&self) -> f32 {
        self.state.lock().unwrap().len_sqrt()
    }
    /// Desired fanout for a given scale
    /// Simulating with sqrt_broadcast_simulate shows we only need to broadcast to sqrt(total_peers) random peers in order to successfully publish to everyone with high probability
    pub fn fanout(&self, scale: f32) -> usize {
        self.state.lock().unwrap().fanout(scale)
    }

    /// Returns channel IDs of removed channels
    pub fn purge(&self, cutoff: SystemTime) -> Vec<ChannelId> {
        let mut guard = self.state.lock().unwrap();
        guard.purge(cutoff)
    }

    pub fn count_by_mode(&self, mode: ChannelMode) -> usize {
        self.state.lock().unwrap().channels.count_by_mode(mode)
    }

    pub(crate) fn count_by_direction(&self, direction: ChannelDirection) -> usize {
        self.state
            .lock()
            .unwrap()
            .channels
            .count_by_direction(direction)
    }

    pub(crate) fn bootstrap_peer(&self) -> SocketAddrV6 {
        self.state.lock().unwrap().bootstrap_peer()
    }

    pub(crate) fn list_realtime_channels(&self, min_version: u8) -> Vec<Arc<Channel>> {
        let mut result = self.state.lock().unwrap().list_realtime(min_version);
        result.sort_by_key(|i| i.remote_addr());
        result
    }

    pub fn port(&self) -> u16 {
        self.port.load(Ordering::SeqCst)
    }

    pub(crate) fn set_port(&self, port: u16) {
        self.port.store(port, Ordering::SeqCst);
    }

    pub(crate) fn create_keepalive_message(&self) -> Message {
        let mut peers = [SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 0, 0, 0); 8];
        self.random_fill_realtime(&mut peers);
        Message::Keepalive(Keepalive { peers })
    }

    pub(crate) fn is_excluded(&self, addr: &SocketAddrV6) -> bool {
        self.state
            .lock()
            .unwrap()
            .excluded_peers
            .is_excluded(addr, self.clock.now())
    }

    pub(crate) fn peer_misbehaved(&self, channel: &Arc<Channel>) {
        {
            // Add to peer exclusion list
            self.state
                .lock()
                .unwrap()
                .excluded_peers
                .peer_misbehaved(&channel.remote_addr(), self.clock.now());
        }

        warn!(peer_addr = ?channel.remote_addr(), mode = ?channel.mode(), direction = ?channel.direction(), "Peer misbehaved!");
        channel.close();
    }

    pub(crate) fn perma_ban(&self, remote_addr: SocketAddrV6) {
        self.state
            .lock()
            .unwrap()
            .excluded_peers
            .perma_ban(remote_addr);
    }

    pub(crate) fn upgrade_to_realtime_connection(
        &self,
        channle_id: ChannelId,
        node_id: Account,
    ) -> bool {
        let (observers, channel) = {
            let state = self.state.lock().unwrap();

            if self.stopped.load(Ordering::SeqCst) {
                return false;
            }

            let Some(channel) = state.channels.get_by_id(channle_id) else {
                return false;
            };

            if let Some(other) = state.channels.get_by_node_id(&node_id) {
                if other.ipv4_address_or_ipv6_subnet() == channel.ipv4_address_or_ipv6_subnet() {
                    // We already have a connection to that node. We allow duplicate node ids, but
                    // only if they come from different IP addresses
                    let endpoint = channel.remote_addr();
                    debug!(
                        node_id = node_id.to_node_id(),
                        remote = %endpoint,
                        "Could not upgrade channel {} to realtime connection, because another channel for the same node ID was found",
                        channel.channel_id(),
                    );
                    drop(state);
                    return false;
                }
            }

            channel.set_node_id(node_id);
            channel.set_mode(ChannelMode::Realtime);

            let observers = state.new_realtime_channel_observers.clone();
            let channel = channel.clone();
            (observers, channel)
        };

        self.stats
            .inc(StatType::TcpChannels, DetailType::ChannelAccepted);

        debug!(
            "Switched to realtime mode (addr: {}, node_id: {})",
            channel.remote_addr(),
            node_id.to_node_id()
        );

        for observer in observers {
            observer(channel.clone());
        }

        true
    }

    pub(crate) fn keepalive(&self) {
        let message = self.create_keepalive_message();

        // Wake up channels
        let to_wake_up = {
            let guard = self.state.lock().unwrap();
            guard.keepalive_list()
        };

        for channel in to_wake_up {
            channel.try_send(&message, BufferDropPolicy::Limiter, TrafficType::Generic);
        }
    }
}

struct State {
    attempts: AttemptContainer,
    channels: ChannelContainer,
    network_constants: NetworkConstants,
    new_realtime_channel_observers: Vec<Arc<dyn Fn(Arc<Channel>) + Send + Sync>>,
    excluded_peers: PeerExclusion,
    stats: Arc<Stats>,
    node_flags: NodeFlags,
    config: TcpConfig,
}

impl State {
    pub fn bootstrap_peer(&mut self) -> SocketAddrV6 {
        let mut peering_endpoint = None;
        let mut channel_id = None;
        for channel in self.channels.iter_by_last_bootstrap_attempt() {
            if channel.mode() == ChannelMode::Realtime
                && channel.network_version() >= self.network_constants.protocol_version_min
            {
                if let Some(peering) = channel.peering_endpoint() {
                    channel_id = Some(channel.channel_id());
                    peering_endpoint = Some(peering);
                    break;
                }
            }
        }

        match (channel_id, peering_endpoint) {
            (Some(id), Some(peering)) => {
                self.channels
                    .set_last_bootstrap_attempt(id, SystemTime::now());
                peering
            }
            _ => SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 0, 0, 0),
        }
    }

    pub fn close_channels(&mut self) {
        for channel in self.channels.iter() {
            channel.close();
        }
        self.channels.clear();
    }

    pub fn purge(&mut self, cutoff: SystemTime) -> Vec<ChannelId> {
        self.channels.close_idle_channels(cutoff);

        // Check if any tcp channels belonging to old protocol versions which may still be alive due to async operations
        self.channels
            .close_old_protocol_versions(self.network_constants.protocol_version_min);

        // Remove channels with dead underlying sockets
        let purged_channel_ids = self.channels.remove_dead();

        // Remove keepalive attempt tracking for attempts older than cutoff
        self.attempts.purge(cutoff);
        purged_channel_ids
    }

    pub fn random_realtime_channels(&self, count: usize, min_version: u8) -> Vec<Arc<Channel>> {
        let mut channels = self.list_realtime(min_version);
        let mut rng = thread_rng();
        channels.shuffle(&mut rng);
        if count > 0 {
            channels.truncate(count)
        }
        channels
    }

    pub fn list_realtime(&self, min_version: u8) -> Vec<Arc<Channel>> {
        self.channels
            .iter()
            .filter(|c| {
                c.network_version() >= min_version
                    && c.is_alive()
                    && c.mode() == ChannelMode::Realtime
            })
            .map(|c| c.clone())
            .collect()
    }

    pub fn keepalive_list(&self) -> Vec<Arc<Channel>> {
        let cutoff = SystemTime::now() - self.network_constants.keepalive_period;
        let mut result = Vec::new();
        for channel in self.channels.iter() {
            if channel.mode() == ChannelMode::Realtime && channel.get_last_packet_sent() < cutoff {
                result.push(channel.clone());
            }
        }

        result
    }

    pub(crate) fn find_channels_by_remote_addr(
        &self,
        remote_addr: &SocketAddrV6,
    ) -> Vec<Arc<Channel>> {
        self.channels
            .get_by_remote_addr(remote_addr)
            .iter()
            .filter(|c| c.is_alive())
            .map(|&c| c.clone())
            .collect()
    }

    pub fn find_realtime_channel_by_remote_addr(
        &self,
        endpoint: &SocketAddrV6,
    ) -> Option<Arc<Channel>> {
        self.channels
            .get_by_remote_addr(endpoint)
            .drain(..)
            .filter(|c| c.mode() == ChannelMode::Realtime && c.is_alive())
            .next()
            .cloned()
    }

    pub fn find_realtime_channel_by_peering_addr(
        &self,
        peering_addr: &SocketAddrV6,
    ) -> Option<Arc<Channel>> {
        self.channels
            .get_by_peering_addr(peering_addr)
            .drain(..)
            .filter(|c| c.mode() == ChannelMode::Realtime && c.is_alive())
            .next()
            .cloned()
    }

    pub(crate) fn find_channels_by_peering_addr(
        &self,
        peering_addr: &SocketAddrV6,
    ) -> Vec<Arc<Channel>> {
        self.channels
            .get_by_peering_addr(peering_addr)
            .iter()
            .filter(|c| c.is_alive())
            .map(|&c| c.clone())
            .collect()
    }

    pub fn find_node_id(&self, node_id: &PublicKey) -> Option<Arc<Channel>> {
        self.channels.get_by_node_id(node_id).map(|c| c.clone())
    }

    pub fn random_fanout_realtime(&self, scale: f32) -> Vec<Arc<Channel>> {
        self.random_realtime_channels(self.fanout(scale), 0)
    }

    pub fn random_fill_realtime(&self, endpoints: &mut [SocketAddrV6]) {
        let mut peers = self.list_realtime(0);
        // Don't include channels with ephemeral remote ports
        peers.retain(|c| c.peering_endpoint().is_some());
        let mut rng = thread_rng();
        peers.shuffle(&mut rng);
        peers.truncate(endpoints.len());

        let null_endpoint = SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 0, 0, 0);

        for (i, target) in endpoints.iter_mut().enumerate() {
            let endpoint = if i < peers.len() {
                peers[i].peering_endpoint().unwrap_or(null_endpoint)
            } else {
                null_endpoint
            };
            *target = endpoint;
        }
    }

    pub fn len_sqrt(&self) -> f32 {
        f32::sqrt(self.channels.count_by_mode(ChannelMode::Realtime) as f32)
    }

    pub fn fanout(&self, scale: f32) -> usize {
        (self.len_sqrt() * scale).ceil() as usize
    }

    pub fn check_limits(
        &mut self,
        peer: &SocketAddrV6,
        direction: ChannelDirection,
        now: Timestamp,
    ) -> AcceptResult {
        if self.excluded_peers.is_excluded(peer, now) {
            self.stats.inc_dir(
                StatType::TcpListenerRejected,
                DetailType::Excluded,
                direction.into(),
            );

            debug!("Rejected connection from excluded peer: {}", peer);
            return AcceptResult::Rejected;
        }

        if !self.node_flags.disable_max_peers_per_ip {
            let count = self.channels.count_by_ip(peer.ip());
            if count >= self.network_constants.max_peers_per_ip {
                self.stats.inc_dir(
                    StatType::TcpListenerRejected,
                    DetailType::MaxPerIp,
                    direction.into(),
                );
                debug!(
                    "Max connections per IP reached ({}, count: {}), unable to open new connection",
                    peer.ip(),
                    count
                );
                return AcceptResult::Rejected;
            }
        }

        // If the address is IPv4 we don't check for a network limit, since its address space isn't big as IPv6/64.
        if !self.node_flags.disable_max_peers_per_subnetwork && !is_ipv4_mapped(&peer.ip()) {
            let subnet = map_address_to_subnetwork(&peer.ip());
            let count = self.channels.count_by_subnet(&subnet);
            if count >= self.network_constants.max_peers_per_subnetwork {
                self.stats.inc_dir(
                    StatType::TcpListenerRejected,
                    DetailType::MaxPerSubnetwork,
                    direction.into(),
                );
                debug!(
                    "Max connections per subnetwork reached ({}), unable to open new connection",
                    peer.ip()
                );
                return AcceptResult::Rejected;
            }
        }

        match direction {
            ChannelDirection::Inbound => {
                let count = self.channels.count_by_direction(ChannelDirection::Inbound);

                if count >= self.config.max_inbound_connections {
                    self.stats.inc_dir(
                        StatType::TcpListenerRejected,
                        DetailType::MaxAttempts,
                        direction.into(),
                    );
                    debug!(
                        "Max inbound connections reached ({}), unable to accept new connection: {}",
                        count,
                        peer.ip()
                    );
                    return AcceptResult::Rejected;
                }
            }
            ChannelDirection::Outbound => {
                let count = self.channels.count_by_direction(ChannelDirection::Outbound);

                if count >= self.config.max_outbound_connections {
                    self.stats.inc_dir(
                        StatType::TcpListenerRejected,
                        DetailType::MaxAttempts,
                        direction.into(),
                    );
                    debug!(
                        "Max outbound connections reached ({}), unable to initiate new connection: {}",
                        count, peer.ip()
                    );
                    return AcceptResult::Rejected;
                }
            }
        }

        AcceptResult::Accepted
    }

    pub fn channels_info(&self) -> ChannelsInfo {
        let mut info = ChannelsInfo::default();
        for entry in self.channels.iter() {
            info.total += 1;
            match entry.mode() {
                ChannelMode::Bootstrap => info.bootstrap += 1,
                ChannelMode::Realtime => info.realtime += 1,
                _ => {}
            }
            match entry.direction() {
                ChannelDirection::Inbound => info.inbound += 1,
                ChannelDirection::Outbound => info.outbound += 1,
            }
        }
        info
    }

    pub fn collect_container_info(&self, name: impl Into<String>) -> ContainerInfoComponent {
        ContainerInfoComponent::Composite(
            name.into(),
            vec![
                ContainerInfoComponent::Leaf(ContainerInfo {
                    name: "channels".to_string(),
                    count: self.channels.len(),
                    sizeof_element: ChannelContainer::ELEMENT_SIZE,
                }),
                ContainerInfoComponent::Leaf(ContainerInfo {
                    name: "attempts".to_string(),
                    count: self.attempts.len(),
                    sizeof_element: AttemptContainer::ELEMENT_SIZE,
                }),
                self.excluded_peers.collect_container_info("excluded_peers"),
            ],
        )
    }
}

#[derive(PartialEq, Eq)]
pub enum AcceptResult {
    Invalid,
    Accepted,
    Rejected,
    Error,
}

#[derive(Default)]
pub(crate) struct ChannelsInfo {
    pub total: usize,
    pub realtime: usize,
    pub bootstrap: usize,
    pub inbound: usize,
    pub outbound: usize,
}
