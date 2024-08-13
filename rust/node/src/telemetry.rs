use rsnano_core::{
    utils::{ContainerInfo, ContainerInfoComponent},
    KeyPair, Signature, WorkVersion,
};
use rsnano_ledger::Ledger;
use rsnano_messages::{Message, TelemetryAck, TelemetryData, TelemetryMaker};
use std::{
    cmp::min,
    collections::{HashMap, VecDeque},
    mem::size_of,
    net::SocketAddrV6,
    sync::{Arc, Condvar, Mutex},
    thread::JoinHandle,
    time::{Duration, Instant, SystemTime},
};

use crate::{
    block_processing::UncheckedMap,
    config::NodeConfig,
    stats::{DetailType, StatType, Stats},
    transport::{BufferDropPolicy, Channel, ChannelMode, Network, TrafficType},
    NetworkParams,
};

/**
 * This class periodically broadcasts and requests telemetry from peers.
 * Those intervals are configurable via `telemetry_request_interval` & `telemetry_broadcast_interval` network constants
 * Telemetry datas are only removed after becoming stale (configurable via `telemetry_cache_cutoff` network constant), so peer data will still be available for a short period after that peer is disconnected
 *
 * Requests can be disabled via `disable_ongoing_telemetry_requests` node flag
 * Broadcasts can be disabled via `disable_providing_telemetry_metrics` node flag
 *
 */
pub struct Telemetry {
    config: TelementryConfig,
    node_config: NodeConfig,
    stats: Arc<Stats>,
    ledger: Arc<Ledger>,
    unchecked: Arc<UncheckedMap>,
    thread: Mutex<Option<JoinHandle<()>>>,
    condition: Condvar,
    mutex: Mutex<TelemetryImpl>,
    network_params: NetworkParams,
    network: Arc<Network>,
    node_id: KeyPair,
    startup_time: Instant,
    notify: Mutex<Vec<Box<dyn Fn(&TelemetryData, &Arc<Channel>) + Send + Sync>>>,
}

impl Telemetry {
    const MAX_SIZE: usize = 1024;

    pub fn new(
        config: TelementryConfig,
        node_config: NodeConfig,
        stats: Arc<Stats>,
        ledger: Arc<Ledger>,
        unchecked: Arc<UncheckedMap>,
        network_params: NetworkParams,
        network: Arc<Network>,
        node_id: KeyPair,
    ) -> Self {
        Self {
            config,
            node_config,
            stats,
            ledger,
            unchecked,
            network_params,
            network,
            thread: Mutex::new(None),
            condition: Condvar::new(),
            mutex: Mutex::new(TelemetryImpl {
                stopped: false,
                triggered: false,
                telemetries: Default::default(),
                last_broadcast: None,
                last_request: None,
            }),
            notify: Mutex::new(Vec::new()),
            node_id,
            startup_time: Instant::now(),
        }
    }

    pub fn stop(&self) {
        self.mutex.lock().unwrap().stopped = true;
        self.condition.notify_all();
        if let Some(handle) = self.thread.lock().unwrap().take() {
            handle.join().unwrap();
        }
    }

    pub fn add_callback(&self, f: Box<dyn Fn(&TelemetryData, &Arc<Channel>) + Send + Sync>) {
        self.notify.lock().unwrap().push(f);
    }

    fn verify(&self, telemetry: &TelemetryAck, channel: &Arc<Channel>) -> bool {
        let Some(data) = &telemetry.0 else {
            self.stats
                .inc(StatType::Telemetry, DetailType::EmptyPayload);
            return false;
        };

        // Check if telemetry node id matches channel node id
        if Some(data.node_id) != channel.get_node_id() {
            self.stats
                .inc(StatType::Telemetry, DetailType::NodeIdMismatch);
            return false;
        }

        // Check whether data is signed by node id presented in telemetry message
        if !data.validate_signature() {
            self.stats
                .inc(StatType::Telemetry, DetailType::InvalidSignature);
            return false;
        }

        if data.genesis_block != self.network_params.ledger.genesis.hash() {
            self.network.peer_misbehaved(channel);

            self.stats
                .inc(StatType::Telemetry, DetailType::GenesisMismatch);
            return false;
        }

        return true; // Telemetry is OK
    }

    /// Process telemetry message from network
    pub fn process(&self, telemetry: &TelemetryAck, channel: &Arc<Channel>) {
        if !self.verify(telemetry, channel) {
            return;
        }
        let data = telemetry.0.as_ref().unwrap();

        let mut guard = self.mutex.lock().unwrap();
        let endpoint = channel.remote_addr();

        if let Some(entry) = guard.telemetries.get_mut(&endpoint) {
            self.stats.inc(StatType::Telemetry, DetailType::Update);
            entry.data = data.clone();
            entry.last_updated = Instant::now();
        } else {
            self.stats.inc(StatType::Telemetry, DetailType::Insert);
            guard.telemetries.push_back(Entry {
                endpoint,
                data: data.clone(),
                last_updated: Instant::now(),
            });

            if guard.telemetries.len() > Self::MAX_SIZE {
                self.stats.inc(StatType::Telemetry, DetailType::Overfill);
                guard.telemetries.pop_front(); // Erase oldest entry
            }
        }

        drop(guard);

        {
            let callbacks = self.notify.lock().unwrap();
            for callback in callbacks.iter() {
                (callback)(data, channel);
            }
        }

        self.stats.inc(StatType::Telemetry, DetailType::Process);
    }

    /// Trigger manual telemetry request to all peers
    pub fn trigger(&self) {
        self.mutex.lock().unwrap().triggered = true;
        self.condition.notify_all();
    }

    pub fn len(&self) -> usize {
        self.mutex.lock().unwrap().telemetries.len()
    }

    fn request_predicate(&self, data: &TelemetryImpl) -> bool {
        if data.triggered {
            return true;
        }
        if self.config.enable_ongoing_requests {
            return data.last_request.is_none()
                || data.last_request.unwrap().elapsed()
                    >= Duration::from_millis(
                        self.network_params.network.telemetry_request_interval_ms as u64,
                    );
        }

        return false;
    }

    fn broadcast_predicate(&self, data: &TelemetryImpl) -> bool {
        if self.config.enable_ongoing_broadcasts {
            return data.last_broadcast.is_none()
                || data.last_broadcast.unwrap().elapsed()
                    >= Duration::from_millis(
                        self.network_params.network.telemetry_broadcast_interval_ms as u64,
                    );
        }

        return false;
    }

    fn run(&self) {
        let mut guard = self.mutex.lock().unwrap();
        while !guard.stopped {
            self.stats.inc(StatType::Telemetry, DetailType::Loop);
            self.cleanup(&mut guard);

            if self.request_predicate(&guard) {
                guard.triggered = false;
                drop(guard);

                self.run_requests();

                guard = self.mutex.lock().unwrap();
                guard.last_request = Some(Instant::now());
            }

            if self.broadcast_predicate(&guard) {
                drop(guard);

                self.run_broadcasts();

                guard = self.mutex.lock().unwrap();
                guard.last_broadcast = Some(Instant::now());
            }

            let wait_duration = min(
                self.network_params.network.telemetry_request_interval_ms,
                self.network_params.network.telemetry_broadcast_interval_ms / 2,
            );
            guard = self
                .condition
                .wait_timeout(guard, Duration::from_millis(wait_duration as u64))
                .unwrap()
                .0
        }
    }

    fn run_requests(&self) {
        let peers = self.network.random_list_realtime(usize::MAX, 0);
        for channel in peers {
            self.request(&channel);
        }
    }

    fn request(&self, channel: &Channel) {
        self.stats.inc(StatType::Telemetry, DetailType::Request);
        channel.try_send(
            &Message::TelemetryReq,
            BufferDropPolicy::Limiter,
            TrafficType::Generic,
        );
    }

    fn run_broadcasts(&self) {
        let telemetry = self.local_telemetry();
        let peers = self.network.random_list_realtime(usize::MAX, 0);
        let message = Message::TelemetryAck(TelemetryAck(Some(telemetry)));
        for channel in peers {
            self.broadcast(&channel, &message);
        }
    }

    fn broadcast(&self, channel: &Channel, message: &Message) {
        self.stats.inc(StatType::Telemetry, DetailType::Broadcast);
        channel.try_send(message, BufferDropPolicy::Limiter, TrafficType::Generic)
    }

    fn cleanup(&self, data: &mut TelemetryImpl) {
        data.telemetries.retain(|entry| {
            // Remove if telemetry data is stale
            if self.has_timed_out(entry) {
                self.stats
                    .inc(StatType::Telemetry, DetailType::CleanupOutdated);
                false // Erase
            } else {
                true // Retain
            }
        })
    }

    fn has_timed_out(&self, entry: &Entry) -> bool {
        entry.last_updated.elapsed()
            > Duration::from_millis(self.network_params.network.telemetry_cache_cutoff_ms as u64)
    }

    /// Returns telemetry for selected endpoint
    pub fn get_telemetry(&self, endpoint: &SocketAddrV6) -> Option<TelemetryData> {
        let guard = self.mutex.lock().unwrap();
        if let Some(entry) = guard.telemetries.get(endpoint) {
            if !self.has_timed_out(entry) {
                return Some(entry.data.clone());
            }
        }
        None
    }

    pub fn get_all_telemetries(&self) -> HashMap<SocketAddrV6, TelemetryData> {
        let guard = self.mutex.lock().unwrap();
        let mut result = HashMap::new();
        for entry in guard.telemetries.iter() {
            if !self.has_timed_out(entry) {
                result.insert(entry.endpoint, entry.data.clone());
            }
        }
        result
    }

    pub fn collect_container_info(&self, name: impl Into<String>) -> ContainerInfoComponent {
        let guard = self.mutex.lock().unwrap();
        ContainerInfoComponent::Composite(
            name.into(),
            vec![ContainerInfoComponent::Leaf(ContainerInfo {
                name: "telemetries".to_string(),
                count: guard.telemetries.len(),
                sizeof_element: OrderedTelemetries::ELEMENT_SIZE,
            })],
        )
    }

    pub fn local_telemetry(&self) -> TelemetryData {
        let mut telemetry_data = TelemetryData {
            node_id: self.node_id.public_key(),
            block_count: self.ledger.block_count(),
            cemented_count: self.ledger.cemented_count(),
            bandwidth_cap: self.node_config.bandwidth_limit as u64,
            protocol_version: self.network_params.network.protocol_version,
            uptime: self.startup_time.elapsed().as_secs(),
            unchecked_count: self.unchecked.len() as u64,
            genesis_block: self.network_params.ledger.genesis.hash(),
            peer_count: self.network.count_by_mode(ChannelMode::Realtime) as u32,
            account_count: self.ledger.account_count(),
            major_version: MAJOR_VERSION,
            minor_version: MINOR_VERSION,
            patch_version: PATCH_VERSION,
            pre_release_version: PRE_RELEASE_VERSION,
            maker: TelemetryMaker::RsNano as u8,
            timestamp: SystemTime::now(),
            active_difficulty: self.network_params.work.threshold_base(WorkVersion::Work1),
            unknown_data: Vec::new(),
            signature: Signature::default(),
        };
        // Make sure this is the final operation!
        telemetry_data.sign(&self.node_id).unwrap();
        telemetry_data
    }
}

pub const MAJOR_VERSION: u8 = 28; // TODO: get this from cmake
pub const MINOR_VERSION: u8 = 0; // TODO: get this from cmake
pub const PATCH_VERSION: u8 = 0; // TODO: get this from cmake
pub const PRE_RELEASE_VERSION: u8 = 99; // TODO: get this from cmake
pub const BUILD_INFO: &'static str = "TODO get buildinfo";
pub const VERSION_STRING: &'static str = "27.0"; // TODO: get this from cmake

#[derive(Clone, Hash, Copy, PartialEq, Eq, Default)]
struct VendorVersion {
    major: u8,
    minor: u8,
    patch: u8,
    pre_release: u8,
    maker: u8,
}

impl Drop for Telemetry {
    fn drop(&mut self) {
        // Thread must be stopped before destruction
        debug_assert!(self.thread.lock().unwrap().is_none());
    }
}

pub struct TelementryConfig {
    pub enable_ongoing_requests: bool,
    pub enable_ongoing_broadcasts: bool,
}

pub trait TelementryExt {
    fn start(&self);
}

struct TelemetryImpl {
    stopped: bool,
    triggered: bool,
    telemetries: OrderedTelemetries,
    last_request: Option<Instant>,
    last_broadcast: Option<Instant>,
}

impl TelementryExt for Arc<Telemetry> {
    fn start(&self) {
        debug_assert!(self.thread.lock().unwrap().is_none());
        let self_l = Arc::clone(self);
        *self.thread.lock().unwrap() = Some(
            std::thread::Builder::new()
                .name("Telemetry".to_string())
                .spawn(move || {
                    self_l.run();
                })
                .unwrap(),
        );
    }
}

struct Entry {
    endpoint: SocketAddrV6,
    data: TelemetryData,
    last_updated: Instant,
}

#[derive(Default)]
struct OrderedTelemetries {
    by_endpoint: HashMap<SocketAddrV6, Entry>,
    sequenced: VecDeque<SocketAddrV6>,
}

impl OrderedTelemetries {
    pub const ELEMENT_SIZE: usize = size_of::<Entry>() + size_of::<SocketAddrV6>() * 2;
    fn len(&self) -> usize {
        self.sequenced.len()
    }

    fn push_back(&mut self, entry: Entry) {
        let endpoint = entry.endpoint;
        if let Some(old) = self.by_endpoint.insert(endpoint, entry) {
            self.sequenced.retain(|i| *i != old.endpoint);
        }
        self.sequenced.push_back(endpoint);
    }

    fn get(&self, entpoint: &SocketAddrV6) -> Option<&Entry> {
        self.by_endpoint.get(entpoint)
    }

    fn get_mut(&mut self, entpoint: &SocketAddrV6) -> Option<&mut Entry> {
        self.by_endpoint.get_mut(entpoint)
    }

    fn pop_front(&mut self) {
        if let Some(endpoint) = self.sequenced.pop_front() {
            self.by_endpoint.remove(&endpoint);
        }
    }

    fn retain(&mut self, mut f: impl FnMut(&Entry) -> bool) {
        self.by_endpoint.retain(|endpoint, entry| {
            let retain = f(entry);
            if !retain {
                self.sequenced.retain(|i| i != endpoint);
            }
            retain
        })
    }

    fn iter(&self) -> impl Iterator<Item = &Entry> {
        self.by_endpoint.values()
    }
}
