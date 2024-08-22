use crate::utils::{
    ipv4_address_or_ipv6_subnet, is_ipv4_mapped, map_address_to_subnetwork, reserved_address,
};

use super::{
    attempt_container::AttemptContainer, ChannelDirection, ChannelId, ChannelMode, PeerExclusion,
    TrafficType,
};
use num::FromPrimitive;
use rand::{seq::SliceRandom, thread_rng};
use rsnano_core::{
    utils::{ContainerInfo, ContainerInfoComponent, TEST_ENDPOINT_1, TEST_ENDPOINT_2},
    Networks, PublicKey,
};
use rsnano_nullable_clock::Timestamp;
use std::{
    collections::HashMap,
    net::{Ipv6Addr, SocketAddrV6},
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tracing::{debug, warn};

/// Default timeout in seconds
const DEFAULT_TIMEOUT: u64 = 120;

pub struct ChannelInfo {
    channel_id: ChannelId,
    local_addr: SocketAddrV6,
    peer_addr: SocketAddrV6,
    data: Mutex<ChannelInfoData>,
    protocol_version: AtomicU8,
    direction: ChannelDirection,

    /// the timestamp (in seconds since epoch) of the last time there was successful activity on the socket
    last_activity: AtomicU64,
    last_bootstrap_attempt: AtomicU64,
    last_packet_received: AtomicU64,
    last_packet_sent: AtomicU64,

    /// Duration in seconds of inactivity that causes a socket timeout
    /// activity is any successful connect, send or receive event
    timeout_seconds: AtomicU64,

    /// Flag that is set when cleanup decides to close the socket due to timeout.
    /// NOTE: Currently used by tcp_server::timeout() but I suspect that this and tcp_server::timeout() are not needed.
    timed_out: AtomicBool,

    /// Set by close() - completion handlers must check this. This is more reliable than checking
    /// error codes as the OS may have already completed the async operation.
    closed: AtomicBool,

    socket_type: AtomicU8,
}

impl ChannelInfo {
    pub fn new(
        channel_id: ChannelId,
        local_addr: SocketAddrV6,
        peer_addr: SocketAddrV6,
        direction: ChannelDirection,
        protocol_version: u8,
        now: Timestamp,
    ) -> Self {
        Self {
            channel_id,
            local_addr,
            peer_addr,
            // TODO set protocol version to 0
            protocol_version: AtomicU8::new(protocol_version),
            direction,
            last_activity: AtomicU64::new(now.into()),
            last_bootstrap_attempt: AtomicU64::new(0),
            last_packet_received: AtomicU64::new(now.into()),
            last_packet_sent: AtomicU64::new(now.into()),
            timeout_seconds: AtomicU64::new(DEFAULT_TIMEOUT),
            timed_out: AtomicBool::new(false),
            socket_type: AtomicU8::new(ChannelMode::Undefined as u8),
            closed: AtomicBool::new(false),
            data: Mutex::new(ChannelInfoData {
                node_id: None,
                is_queue_full_impl: None,
                peering_addr: if direction == ChannelDirection::Outbound {
                    Some(peer_addr)
                } else {
                    None
                },
            }),
        }
    }

    pub fn new_test_instance() -> Self {
        Self::new(
            ChannelId::from(42),
            TEST_ENDPOINT_1,
            TEST_ENDPOINT_2,
            ChannelDirection::Outbound,
            u8::MAX,
            Timestamp::new_test_instance(),
        )
    }

    pub fn set_queue_full_query(&self, query: Box<dyn Fn(TrafficType) -> bool + Send>) {
        self.data.lock().unwrap().is_queue_full_impl = Some(query);
    }

    pub fn channel_id(&self) -> ChannelId {
        self.channel_id
    }

    pub fn node_id(&self) -> Option<PublicKey> {
        self.data.lock().unwrap().node_id
    }

    pub fn direction(&self) -> ChannelDirection {
        self.direction
    }

    pub fn local_addr(&self) -> SocketAddrV6 {
        self.local_addr
    }

    /// The address that we are connected to. If this is an incoming channel, then
    /// the peer_addr uses an ephemeral port
    pub fn peer_addr(&self) -> SocketAddrV6 {
        self.peer_addr
    }

    /// The address where the peer accepts incoming connections. In case of an outbound
    /// channel, the peer_addr and peering_addr are the same
    pub fn peering_addr(&self) -> Option<SocketAddrV6> {
        self.data.lock().unwrap().peering_addr.clone()
    }

    pub fn peering_addr_or_peer_addr(&self) -> SocketAddrV6 {
        self.data
            .lock()
            .unwrap()
            .peering_addr
            .clone()
            .unwrap_or(self.peer_addr())
    }

    pub fn ipv4_address_or_ipv6_subnet(&self) -> Ipv6Addr {
        ipv4_address_or_ipv6_subnet(&self.peer_addr().ip())
    }

    pub fn subnetwork(&self) -> Ipv6Addr {
        map_address_to_subnetwork(self.peer_addr().ip())
    }

    pub fn protocol_version(&self) -> u8 {
        self.protocol_version.load(Ordering::Relaxed)
    }

    // TODO make private and set via NetworkInfo
    pub fn set_protocol_version(&self, version: u8) {
        self.protocol_version.store(version, Ordering::Relaxed);
    }

    pub fn last_activity(&self) -> Timestamp {
        self.last_activity.load(Ordering::Relaxed).into()
    }

    pub fn set_last_activity(&self, now: Timestamp) {
        self.last_activity.store(now.into(), Ordering::Relaxed);
    }

    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_seconds.load(Ordering::Relaxed))
    }

    pub fn set_timeout(&self, value: Duration) {
        self.timeout_seconds
            .store(value.as_secs(), Ordering::Relaxed)
    }

    pub fn timed_out(&self) -> bool {
        self.timed_out.load(Ordering::Relaxed)
    }

    pub fn set_timed_out(&self, value: bool) {
        self.timed_out.store(value, Ordering::Relaxed)
    }

    pub fn is_alive(&self) -> bool {
        !self.is_closed()
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Relaxed)
    }

    pub fn close(&self) {
        self.closed.store(true, Ordering::Relaxed);
        self.set_timeout(Duration::ZERO);
    }

    pub fn set_node_id(&self, node_id: PublicKey) {
        self.data.lock().unwrap().node_id = Some(node_id);
    }

    pub fn set_peering_addr(&self, peering_addr: SocketAddrV6) {
        self.data.lock().unwrap().peering_addr = Some(peering_addr);
    }

    pub fn mode(&self) -> ChannelMode {
        FromPrimitive::from_u8(self.socket_type.load(Ordering::SeqCst)).unwrap()
    }

    pub fn set_mode(&self, mode: ChannelMode) {
        self.socket_type.store(mode as u8, Ordering::SeqCst);
    }

    pub fn last_bootstrap_attempt(&self) -> Timestamp {
        self.last_bootstrap_attempt.load(Ordering::Relaxed).into()
    }

    pub fn set_last_bootstrap_attempt(&self, now: Timestamp) {
        self.last_bootstrap_attempt
            .store(now.into(), Ordering::Relaxed);
    }

    pub fn last_packet_received(&self) -> Timestamp {
        self.last_packet_received.load(Ordering::Relaxed).into()
    }

    pub fn set_last_packet_received(&self, now: Timestamp) {
        self.last_packet_received
            .store(now.into(), Ordering::Relaxed);
    }

    pub fn last_packet_sent(&self) -> Timestamp {
        self.last_packet_sent.load(Ordering::Relaxed).into()
    }

    pub fn set_last_packet_sent(&self, now: Timestamp) {
        self.last_packet_sent.store(now.into(), Ordering::Relaxed);
    }

    pub fn is_queue_full(&self, traffic_type: TrafficType) -> bool {
        let guard = self.data.lock().unwrap();
        match &guard.is_queue_full_impl {
            Some(cb) => cb(traffic_type),
            None => false,
        }
    }
}

struct ChannelInfoData {
    node_id: Option<PublicKey>,
    peering_addr: Option<SocketAddrV6>,
    is_queue_full_impl: Option<Box<dyn Fn(TrafficType) -> bool + Send>>,
}

pub struct NetworkConfig {
    pub max_inbound_connections: usize,
    pub max_outbound_connections: usize,

    /** Maximum number of peers per IP. It is also the max number of connections per IP*/
    pub max_peers_per_ip: usize,

    /** Maximum number of peers per subnetwork */
    pub max_peers_per_subnetwork: usize,
    pub max_attempts_per_ip: usize,

    pub allow_local_peers: bool,
    pub min_protocol_version: u8,
    pub disable_max_peers_per_ip: bool,         // For testing only
    pub disable_max_peers_per_subnetwork: bool, // For testing only
    pub disable_network: bool,
    pub listening_port: u16,
    pub default_protocol_version: u8,
}

impl NetworkConfig {
    pub(crate) fn default_for(network: Networks) -> Self {
        let is_dev = network == Networks::NanoDevNetwork;
        Self {
            max_inbound_connections: if is_dev { 128 } else { 2048 },
            max_outbound_connections: if is_dev { 128 } else { 2048 },
            allow_local_peers: true,
            max_peers_per_ip: match network {
                Networks::NanoDevNetwork | Networks::NanoBetaNetwork => 256,
                _ => 4,
            },
            max_peers_per_subnetwork: match network {
                Networks::NanoDevNetwork | Networks::NanoBetaNetwork => 256,
                _ => 16,
            },
            max_attempts_per_ip: if is_dev { 128 } else { 1 },
            min_protocol_version: 0x14,     //TODO don't hard code
            default_protocol_version: 0x15, //TODO don't hard code
            disable_max_peers_per_ip: false,
            disable_max_peers_per_subnetwork: false,
            disable_network: false,
            listening_port: match network {
                Networks::NanoDevNetwork => 44000,
                Networks::NanoBetaNetwork => 54000,
                Networks::NanoTestNetwork => 17076,
                _ => 7075,
            },
        }
    }
}

pub struct NetworkInfo {
    next_channel_id: usize,
    channels: HashMap<ChannelId, Arc<ChannelInfo>>,
    stopped: bool,
    new_realtime_channel_observers: Vec<Arc<dyn Fn(Arc<ChannelInfo>) + Send + Sync>>,
    attempts: AttemptContainer,
    network_config: NetworkConfig,
    excluded_peers: PeerExclusion,
}

#[derive(Debug, Clone, Copy)]
pub enum NetworkError {
    MaxConnections,
    MaxConnectionsPerSubnetwork,
    MaxConnectionsPerIp,
    /// Peer is excluded due to bad behavior
    PeerExcluded,
    InvalidIp,
    /// We are already connected to that peer and we tried to connect a second time
    DuplicateConnection,
}

impl NetworkInfo {
    pub fn new(network_config: NetworkConfig) -> Self {
        Self {
            next_channel_id: 1,
            channels: HashMap::new(),
            stopped: false,
            new_realtime_channel_observers: Vec::new(),
            attempts: Default::default(),
            network_config,
            excluded_peers: PeerExclusion::new(),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn new_test_instance() -> Self {
        Self::new(NetworkConfig::default_for(Networks::NanoDevNetwork))
    }

    pub(crate) fn on_new_realtime_channel(
        &mut self,
        callback: Arc<dyn Fn(Arc<ChannelInfo>) + Send + Sync>,
    ) {
        self.new_realtime_channel_observers.push(callback);
    }

    pub(crate) fn new_realtime_channel_observers(
        &self,
    ) -> Vec<Arc<dyn Fn(Arc<ChannelInfo>) + Send + Sync>> {
        self.new_realtime_channel_observers.clone()
    }

    pub fn is_inbound_slot_available(&self) -> bool {
        self.count_by_direction(ChannelDirection::Inbound)
            < self.network_config.max_inbound_connections
    }

    /// Perma bans are used for prohibiting a node to connect to itself.
    pub(crate) fn perma_ban(&mut self, peer_addr: SocketAddrV6) {
        self.excluded_peers.perma_ban(peer_addr);
    }

    pub(crate) fn is_excluded(&mut self, peer_addr: &SocketAddrV6, now: Timestamp) -> bool {
        self.excluded_peers.is_excluded(peer_addr, now)
    }

    pub(crate) fn add_outbound_attempt(
        &mut self,
        peer: SocketAddrV6,
        planned_mode: ChannelMode,
        now: Timestamp,
    ) -> Result<(), NetworkError> {
        self.validate_new_connection(&peer, ChannelDirection::Outbound, planned_mode, now)?;
        self.attempts.insert(peer, ChannelDirection::Outbound, now);
        Ok(())
    }

    pub(crate) fn remove_attempt(&mut self, remote: &SocketAddrV6) {
        self.attempts.remove(&remote);
    }

    pub fn add(
        &mut self,
        local_addr: SocketAddrV6,
        peer_addr: SocketAddrV6,
        direction: ChannelDirection,
        planned_mode: ChannelMode,
        now: Timestamp,
    ) -> Result<Arc<ChannelInfo>, NetworkError> {
        self.validate_new_connection(&peer_addr, direction, planned_mode, now)?;
        let channel_id = self.get_next_channel_id();
        let channel_info = Arc::new(ChannelInfo::new(
            channel_id,
            local_addr,
            peer_addr,
            direction,
            self.network_config.default_protocol_version,
            now,
        ));
        self.channels.insert(channel_id, channel_info.clone());
        Ok(channel_info)
    }

    fn get_next_channel_id(&mut self) -> ChannelId {
        let id = self.next_channel_id.into();
        self.next_channel_id += 1;
        id
    }

    pub fn listening_port(&self) -> u16 {
        self.network_config.listening_port
    }

    pub fn set_listening_port(&mut self, port: u16) {
        self.network_config.listening_port = port
    }

    pub fn get(&self, channel_id: ChannelId) -> Option<&Arc<ChannelInfo>> {
        self.channels.get(&channel_id)
    }

    pub fn remove(&mut self, channel_id: ChannelId) {
        self.channels.remove(&channel_id);
    }

    pub fn set_node_id(&self, channel_id: ChannelId, node_id: PublicKey) {
        if let Some(channel) = self.channels.get(&channel_id) {
            channel.set_node_id(node_id);
        }
    }

    pub fn find_node_id(&self, node_id: &PublicKey) -> Option<&Arc<ChannelInfo>> {
        self.channels
            .values()
            .find(|c| c.node_id() == Some(*node_id) && c.is_alive())
    }

    pub fn find_realtime_channel_by_remote_addr(
        &self,
        endpoint: &SocketAddrV6,
    ) -> Option<&Arc<ChannelInfo>> {
        self.channels.values().find(|c| {
            c.mode() == ChannelMode::Realtime && c.is_alive() && c.peer_addr() == *endpoint
        })
    }

    pub(crate) fn find_realtime_channel_by_peering_addr(
        &self,
        peering_addr: &SocketAddrV6,
    ) -> Option<ChannelId> {
        self.channels
            .values()
            .find(|c| {
                c.mode() == ChannelMode::Realtime
                    && c.is_alive()
                    && c.peering_addr() == Some(*peering_addr)
            })
            .map(|c| c.channel_id())
    }

    pub fn random_realtime_channels(&self, count: usize, min_version: u8) -> Vec<Arc<ChannelInfo>> {
        let mut channels = self.list_realtime(min_version);
        let mut rng = thread_rng();
        channels.shuffle(&mut rng);
        if count > 0 {
            channels.truncate(count)
        }
        channels
    }

    pub fn random_fanout_realtime(&self, scale: f32) -> Vec<Arc<ChannelInfo>> {
        self.random_realtime_channels(self.fanout(scale), 0)
    }

    pub fn list_realtime(&self, min_version: u8) -> Vec<Arc<ChannelInfo>> {
        self.channels
            .values()
            .filter(|c| {
                c.protocol_version() >= min_version
                    && c.is_alive()
                    && c.mode() == ChannelMode::Realtime
            })
            .map(|c| c.clone())
            .collect()
    }

    pub(crate) fn list_realtime_channels(&self, min_version: u8) -> Vec<Arc<ChannelInfo>> {
        let mut result = self.list_realtime(min_version);
        result.sort_by_key(|i| i.peer_addr());
        result
    }

    pub fn not_a_peer(&self, endpoint: &SocketAddrV6, allow_local_peers: bool) -> bool {
        endpoint.ip().is_unspecified()
            || reserved_address(endpoint, allow_local_peers)
            || endpoint == &SocketAddrV6::new(Ipv6Addr::LOCALHOST, self.listening_port(), 0, 0)
    }

    pub(crate) fn random_list_realtime(
        &self,
        count: usize,
        min_version: u8,
    ) -> Vec<Arc<ChannelInfo>> {
        let mut channels = self.list_realtime(min_version);
        let mut rng = thread_rng();
        channels.shuffle(&mut rng);
        if count > 0 {
            channels.truncate(count)
        }
        channels
    }

    pub(crate) fn random_list_realtime_ids(&self) -> Vec<ChannelId> {
        self.random_list_realtime(usize::MAX, 0)
            .iter()
            .map(|c| c.channel_id())
            .collect()
    }

    /// Returns channel IDs of removed channels
    pub fn purge(&mut self, now: Timestamp, cutoff_period: Duration) -> Vec<ChannelId> {
        self.close_idle_channels(now, cutoff_period);

        // Check if any tcp channels belonging to old protocol versions which may still be alive due to async operations
        self.close_old_protocol_versions(self.network_config.min_protocol_version);

        // Remove channels with dead underlying sockets
        let purged_channel_ids = self.remove_dead_channels();

        // Remove keepalive attempt tracking for attempts older than cutoff
        self.attempts.purge(now, cutoff_period);
        purged_channel_ids
    }

    fn close_idle_channels(&mut self, now: Timestamp, cutoff_period: Duration) {
        for entry in self.channels.values() {
            if now - entry.last_packet_sent() >= cutoff_period {
                debug!(remote_addr = ?entry.peer_addr(), channel_id = %entry.channel_id(), mode = ?entry.mode(), "Closing idle channel");
                entry.close();
            }
        }
    }

    fn close_old_protocol_versions(&mut self, min_version: u8) {
        for channel in self.channels.values() {
            if channel.protocol_version() < min_version {
                debug!(channel_id = %channel.channel_id(), peer_addr = ?channel.peer_addr(), version = channel.protocol_version(), min_version,
                    "Closing channel with old protocol version",
                );
                channel.close();
            }
        }
    }

    /// Removes dead channels and returns their channel ids
    fn remove_dead_channels(&mut self) -> Vec<ChannelId> {
        let dead_channels: Vec<_> = self
            .channels
            .values()
            .filter(|c| !c.is_alive())
            .cloned()
            .collect();

        for channel in &dead_channels {
            debug!("Removing dead channel: {}", channel.peer_addr());
            self.channels.remove(&channel.channel_id());
        }

        dead_channels.iter().map(|c| c.channel_id()).collect()
    }

    pub(crate) fn is_queue_full(&self, channel_id: ChannelId, traffic_type: TrafficType) -> bool {
        self.channels
            .get(&channel_id)
            .map(|c| c.is_queue_full(traffic_type))
            .unwrap_or(true)
    }

    fn len_sqrt(&self) -> f32 {
        f32::sqrt(self.count_by_mode(ChannelMode::Realtime) as f32)
    }

    /// Desired fanout for a given scale
    /// Simulating with sqrt_broadcast_simulate shows we only need to broadcast to sqrt(total_peers) random peers in order to successfully publish to everyone with high probability
    pub fn fanout(&self, scale: f32) -> usize {
        (self.len_sqrt() * scale).ceil() as usize
    }

    fn count_by_ip(&self, ip: &Ipv6Addr) -> usize {
        self.channels
            .values()
            .filter(|c| c.is_alive() && c.ipv4_address_or_ipv6_subnet() == *ip)
            .count()
    }

    fn count_by_subnet(&self, subnet: &Ipv6Addr) -> usize {
        self.channels
            .values()
            .filter(|c| c.is_alive() && c.subnetwork() == *subnet)
            .count()
    }

    pub(crate) fn count_by_direction(&self, direction: ChannelDirection) -> usize {
        self.channels
            .values()
            .filter(|c| c.is_alive() && c.direction() == direction)
            .count()
    }

    pub fn count_by_mode(&self, mode: ChannelMode) -> usize {
        self.channels
            .values()
            .filter(|c| c.is_alive() && c.mode() == mode)
            .count()
    }

    pub fn bootstrap_peer(&mut self, now: Timestamp) -> SocketAddrV6 {
        let mut peering_endpoint = None;
        let mut channel = None;
        for i in self.iter_by_last_bootstrap_attempt() {
            if i.mode() == ChannelMode::Realtime
                && i.protocol_version() >= self.network_config.min_protocol_version
            {
                if let Some(peering) = i.peering_addr() {
                    channel = Some(i);
                    peering_endpoint = Some(peering);
                    break;
                }
            }
        }

        match (channel, peering_endpoint) {
            (Some(c), Some(peering)) => {
                c.set_last_bootstrap_attempt(now);
                peering
            }
            _ => SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 0, 0, 0),
        }
    }

    pub fn iter_by_last_bootstrap_attempt(&self) -> Vec<Arc<ChannelInfo>> {
        let mut channels: Vec<_> = self
            .channels
            .values()
            .filter(|c| c.is_alive())
            .cloned()
            .collect();
        channels.sort_by(|a, b| a.last_bootstrap_attempt().cmp(&b.last_bootstrap_attempt()));
        channels
    }

    pub(crate) fn find_channels_by_remote_addr(
        &self,
        remote_addr: &SocketAddrV6,
    ) -> Vec<Arc<ChannelInfo>> {
        self.channels
            .values()
            .filter(|c| c.is_alive() && c.peer_addr() == *remote_addr)
            .cloned()
            .collect()
    }

    pub(crate) fn find_channels_by_peering_addr(
        &self,
        peering_addr: &SocketAddrV6,
    ) -> Vec<Arc<ChannelInfo>> {
        self.channels
            .values()
            .filter(|c| c.is_alive() && c.peering_addr() == Some(*peering_addr))
            .cloned()
            .collect()
    }

    fn max_ip_connections(&self, endpoint: &SocketAddrV6) -> bool {
        if self.network_config.disable_max_peers_per_ip {
            return false;
        }
        let count =
            self.count_by_ip(&endpoint.ip()) + self.attempts.count_by_address(&endpoint.ip());
        count >= self.network_config.max_peers_per_ip
    }

    fn max_subnetwork_connections(&self, peer: &SocketAddrV6) -> bool {
        if self.network_config.disable_max_peers_per_subnetwork {
            return false;
        }

        // If the address is IPv4 we don't check for a network limit, since its address space isn't big as IPv6/64.
        if is_ipv4_mapped(&peer.ip()) {
            return false;
        }

        let subnet = map_address_to_subnetwork(peer.ip());
        let subnet_count =
            self.count_by_subnet(&subnet) + self.attempts.count_by_subnetwork(&subnet);

        subnet_count >= self.network_config.max_peers_per_subnetwork
    }

    pub fn validate_new_connection(
        &mut self,
        peer: &SocketAddrV6,
        direction: ChannelDirection,
        planned_mode: ChannelMode,
        now: Timestamp,
    ) -> Result<(), NetworkError> {
        if self.network_config.disable_network {
            return Err(NetworkError::MaxConnections);
        }

        let count = self.count_by_direction(direction);
        if count >= self.max_connections(direction) {
            return Err(NetworkError::MaxConnections);
        }

        if self.excluded_peers.is_excluded(peer, now) {
            return Err(NetworkError::PeerExcluded);
        }

        if !self.network_config.disable_max_peers_per_ip {
            let count = self.count_by_ip(peer.ip());
            if count >= self.network_config.max_peers_per_ip {
                return Err(NetworkError::MaxConnectionsPerIp);
            }
        }

        // Don't overload single IP
        if self.max_ip_connections(peer) {
            return Err(NetworkError::MaxConnectionsPerIp);
        }

        if self.max_subnetwork_connections(peer) {
            return Err(NetworkError::MaxConnectionsPerSubnetwork);
        }

        // Don't contact invalid IPs
        if self.not_a_peer(peer, self.network_config.allow_local_peers) {
            return Err(NetworkError::InvalidIp);
        }

        if direction == ChannelDirection::Outbound {
            // Don't connect to nodes that already sent us something
            if self
                .find_channels_by_remote_addr(peer)
                .iter()
                .any(|c| c.mode() == planned_mode || c.mode() == ChannelMode::Undefined)
            {
                return Err(NetworkError::DuplicateConnection);
            }
            if self
                .find_channels_by_peering_addr(peer)
                .iter()
                .any(|c| c.mode() == planned_mode || c.mode() == ChannelMode::Undefined)
            {
                return Err(NetworkError::DuplicateConnection);
            }
        }

        Ok(())
    }

    fn max_connections(&self, direction: ChannelDirection) -> usize {
        match direction {
            ChannelDirection::Inbound => self.network_config.max_inbound_connections,
            ChannelDirection::Outbound => self.network_config.max_outbound_connections,
        }
    }

    pub(crate) fn set_peering_addr(&self, channel_id: ChannelId, peering_addr: SocketAddrV6) {
        if let Some(channel) = self.channels.get(&channel_id) {
            channel.set_peering_addr(peering_addr);
        }
    }

    pub(crate) fn peer_misbehaved(&mut self, channel_id: ChannelId, now: Timestamp) {
        let Some(channel) = self.channels.get(&channel_id) else {
            return;
        };
        let channel = channel.clone();

        // Add to peer exclusion list

        self.excluded_peers
            .peer_misbehaved(&channel.peer_addr(), now);

        let peer_addr = channel.peer_addr();
        let mode = channel.mode();
        let direction = channel.direction();

        channel.close();
        warn!(?peer_addr, ?mode, ?direction, "Peer misbehaved!");
    }

    pub fn close(&mut self) {}

    pub fn stop(&mut self) -> bool {
        if self.stopped {
            false
        } else {
            for channel in self.channels.values() {
                channel.close();
            }
            self.channels.clear();
            self.stopped = true;
            true
        }
    }

    pub fn random_fill_realtime(&self, endpoints: &mut [SocketAddrV6]) {
        let mut peers = self.list_realtime(0);
        // Don't include channels with ephemeral remote ports
        peers.retain(|c| c.peering_addr().is_some());
        let mut rng = thread_rng();
        peers.shuffle(&mut rng);
        peers.truncate(endpoints.len());

        let null_endpoint = SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 0, 0, 0);

        for (i, target) in endpoints.iter_mut().enumerate() {
            let endpoint = if i < peers.len() {
                peers[i].peering_addr().unwrap_or(null_endpoint)
            } else {
                null_endpoint
            };
            *target = endpoint;
        }
    }

    pub(crate) fn set_protocol_version(&self, channel_id: ChannelId, protocol_version: u8) {
        if let Some(channel) = self.channels.get(&channel_id) {
            channel.set_protocol_version(protocol_version)
        }
    }

    pub(crate) fn upgrade_to_realtime_connection(
        &self,
        channel_id: ChannelId,
        node_id: PublicKey,
    ) -> bool {
        let (observers, channel) = {
            if self.is_stopped() {
                return false;
            }

            let Some(channel) = self.channels.get(&channel_id) else {
                return false;
            };

            if let Some(other) = self.find_node_id(&node_id) {
                if other.ipv4_address_or_ipv6_subnet() == channel.ipv4_address_or_ipv6_subnet() {
                    // We already have a connection to that node. We allow duplicate node ids, but
                    // only if they come from different IP addresses
                    return false;
                }
            }

            channel.set_node_id(node_id);
            channel.set_mode(ChannelMode::Realtime);

            let observers = self.new_realtime_channel_observers();
            let channel = channel.clone();
            (observers, channel)
        };

        for observer in observers {
            observer(channel.clone());
        }

        true
    }

    pub fn idle_channels(&self, min_idle_time: Duration, now: Timestamp) -> Vec<ChannelId> {
        let mut result = Vec::new();
        for channel in self.channels.values() {
            if channel.mode() == ChannelMode::Realtime
                && now - channel.last_packet_sent() >= min_idle_time
            {
                result.push(channel.channel_id());
            }
        }

        result
    }

    pub(crate) fn channels_info(&self) -> ChannelsInfo {
        let mut info = ChannelsInfo::default();
        for channel in self.channels.values() {
            info.total += 1;
            match channel.mode() {
                ChannelMode::Bootstrap => info.bootstrap += 1,
                ChannelMode::Realtime => info.realtime += 1,
                _ => {}
            }
            match channel.direction() {
                ChannelDirection::Inbound => info.inbound += 1,
                ChannelDirection::Outbound => info.outbound += 1,
            }
        }
        info
    }

    #[allow(dead_code)]
    pub(crate) fn len(&self) -> usize {
        self.channels.len()
    }

    pub fn is_stopped(&self) -> bool {
        self.stopped
    }

    pub fn collect_container_info(&self, name: impl Into<String>) -> ContainerInfoComponent {
        ContainerInfoComponent::Composite(
            name.into(),
            vec![
                ContainerInfoComponent::Leaf(ContainerInfo {
                    name: "channels".to_string(),
                    count: self.channels.len(),
                    sizeof_element: size_of::<Arc<ChannelInfo>>(),
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

impl Drop for NetworkInfo {
    fn drop(&mut self) {
        self.stop();
    }
}

#[derive(Default)]
pub(crate) struct ChannelsInfo {
    pub total: usize,
    pub realtime: usize,
    pub bootstrap: usize,
    pub inbound: usize,
    pub outbound: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsnano_core::utils::{NULL_ENDPOINT, TEST_ENDPOINT_3};

    #[test]
    fn newly_added_channel_is_not_a_realtime_channel() {
        let mut network = NetworkInfo::new_test_instance();
        network
            .add(
                TEST_ENDPOINT_1,
                TEST_ENDPOINT_2,
                ChannelDirection::Inbound,
                ChannelMode::Realtime,
                Timestamp::new_test_instance(),
            )
            .unwrap();
        assert_eq!(network.list_realtime_channels(0).len(), 0);
    }

    #[test]
    fn reserved_ip_is_not_a_peer() {
        let network = NetworkInfo::new_test_instance();

        assert!(network.not_a_peer(
            &SocketAddrV6::new(Ipv6Addr::new(0xff00u16, 0, 0, 0, 0, 0, 0, 0), 1000, 0, 0),
            true
        ));
        assert!(network.not_a_peer(&SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 10000, 0, 0), true));
        assert!(network.not_a_peer(
            &SocketAddrV6::new(Ipv6Addr::LOCALHOST, network.listening_port(), 0, 0),
            false
        ));

        // Test with a valid IP address
        assert_eq!(
            network.not_a_peer(
                &SocketAddrV6::new(Ipv6Addr::from_bits(0x08080808), 10000, 0, 0),
                true
            ),
            false
        );
    }

    #[test]
    fn upgrade_channel_to_realtime_channel() {
        let mut network = NetworkInfo::new_test_instance();
        let channel = network
            .add(
                TEST_ENDPOINT_1,
                TEST_ENDPOINT_2,
                ChannelDirection::Inbound,
                ChannelMode::Realtime,
                Timestamp::new_test_instance(),
            )
            .unwrap();

        assert!(network.upgrade_to_realtime_connection(channel.channel_id(), PublicKey::from(456)));
        assert_eq!(network.list_realtime_channels(0).len(), 1);
    }

    #[test]
    fn random_fill_peering_endpoints_empty() {
        let network = NetworkInfo::new_test_instance();
        let mut endpoints = [NULL_ENDPOINT; 3];
        network.random_fill_realtime(&mut endpoints);
        assert_eq!(endpoints, [NULL_ENDPOINT; 3]);
    }

    #[test]
    fn random_fill_peering_endpoints_part() {
        let mut network = NetworkInfo::new_test_instance();
        add_realtime_channel_with_peering_addr(&mut network, TEST_ENDPOINT_1);
        add_realtime_channel_with_peering_addr(&mut network, TEST_ENDPOINT_2);
        let mut endpoints = [NULL_ENDPOINT; 3];
        network.random_fill_realtime(&mut endpoints);
        assert!(endpoints.contains(&TEST_ENDPOINT_1));
        assert!(endpoints.contains(&TEST_ENDPOINT_2));
        assert_eq!(endpoints[2], NULL_ENDPOINT);
    }

    #[test]
    fn random_fill_peering_endpoints() {
        let mut network = NetworkInfo::new_test_instance();
        add_realtime_channel_with_peering_addr(&mut network, TEST_ENDPOINT_1);
        add_realtime_channel_with_peering_addr(&mut network, TEST_ENDPOINT_2);
        add_realtime_channel_with_peering_addr(&mut network, TEST_ENDPOINT_3);
        let mut endpoints = [NULL_ENDPOINT; 3];
        network.random_fill_realtime(&mut endpoints);
        assert!(endpoints.contains(&TEST_ENDPOINT_1));
        assert!(endpoints.contains(&TEST_ENDPOINT_2));
        assert!(endpoints.contains(&TEST_ENDPOINT_3));
    }

    fn add_realtime_channel_with_peering_addr(
        network: &mut NetworkInfo,
        peering_addr: SocketAddrV6,
    ) {
        let channel = network
            .add(
                TEST_ENDPOINT_1,
                peering_addr,
                ChannelDirection::Inbound,
                ChannelMode::Realtime,
                Timestamp::new_test_instance(),
            )
            .unwrap();
        channel.set_peering_addr(peering_addr);
        network.upgrade_to_realtime_connection(
            channel.channel_id(),
            PublicKey::from(peering_addr.ip().to_bits()),
        );
    }

    mod purging {
        use super::*;

        #[test]
        fn purge_empty() {
            let mut network = NetworkInfo::new_test_instance();
            network.purge(Timestamp::new_test_instance(), Duration::from_secs(1));
            assert_eq!(network.len(), 0);
        }

        #[test]
        fn dont_purge_new_channel() {
            let mut network = NetworkInfo::new_test_instance();
            let now = Timestamp::new_test_instance();
            network
                .add(
                    TEST_ENDPOINT_1,
                    TEST_ENDPOINT_2,
                    ChannelDirection::Outbound,
                    ChannelMode::Realtime,
                    now,
                )
                .unwrap();
            network.purge(now, Duration::from_secs(1));
            assert_eq!(network.len(), 1);
        }

        #[test]
        fn purge_if_last_packet_sent_is_above_timeout() {
            let mut network = NetworkInfo::new_test_instance();
            let now = Timestamp::new_test_instance();
            let channel = network
                .add(
                    TEST_ENDPOINT_1,
                    TEST_ENDPOINT_2,
                    ChannelDirection::Outbound,
                    ChannelMode::Realtime,
                    now,
                )
                .unwrap();
            channel.set_last_packet_sent(now - Duration::from_secs(300));
            network.purge(now, Duration::from_secs(1));
            assert_eq!(network.len(), 0);
        }

        #[test]
        fn dont_purge_if_packet_sent_within_timeout() {
            let mut network = NetworkInfo::new_test_instance();
            let now = Timestamp::new_test_instance();
            let channel = network
                .add(
                    TEST_ENDPOINT_1,
                    TEST_ENDPOINT_2,
                    ChannelDirection::Outbound,
                    ChannelMode::Realtime,
                    now,
                )
                .unwrap();
            channel.set_last_packet_sent(now);
            network.purge(now, Duration::from_secs(1));
            assert_eq!(network.len(), 1);
        }
    }
}
