use super::{
    bootstrap_limits, BootstrapAttempts, BootstrapClient, BootstrapInitiator,
    BootstrapInitiatorConfig, BootstrapMode, BootstrapStrategy, BulkPullClient,
    BulkPullClientConfig, BulkPullClientExt, PullInfo, PullsCache,
};
use crate::{
    block_processing::BlockProcessor,
    stats::{DetailType, Direction, StatType, Stats},
    transport::{
        ChannelDirection, ChannelEnum, ChannelTcp, Network, NullSocketObserver,
        OutboundBandwidthLimiter, SocketBuilder, SocketExtensions, SocketObserver,
    },
    utils::{into_ipv6_socket_address, AsyncRuntime, ThreadPool, ThreadPoolImpl},
};
use ordered_float::OrderedFloat;
use rsnano_core::{utils::PropertyTree, Account, BlockHash};
use std::{
    cmp::{max, min},
    collections::{BinaryHeap, HashSet, VecDeque},
    net::{Ipv6Addr, SocketAddrV6},
    ops::Deref,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc, Condvar, Mutex, MutexGuard, Weak,
    },
    time::{Duration, SystemTime},
};
use tracing::debug;

/// Container for bootstrap_client objects. Owned by bootstrap_initiator which pools open connections and makes them available
/// for use by different bootstrap sessions.
pub struct BootstrapConnections {
    condition: Condvar,
    populate_connections_started: AtomicBool,
    attempts: Arc<Mutex<BootstrapAttempts>>,
    mutex: Mutex<BootstrapConnectionsData>,
    config: BootstrapInitiatorConfig,
    pub connections_count: AtomicU32,
    new_connections_empty: AtomicBool,
    stopped: AtomicBool,
    network: Arc<Network>,
    workers: Arc<dyn ThreadPool>,
    async_rt: Arc<AsyncRuntime>,
    socket_observer: Arc<dyn SocketObserver>,
    stats: Arc<Stats>,
    block_processor: Arc<BlockProcessor>,
    outbound_limiter: Arc<OutboundBandwidthLimiter>,
    bootstrap_initiator: Mutex<Option<Weak<BootstrapInitiator>>>,
    pulls_cache: Arc<Mutex<PullsCache>>,
}

impl BootstrapConnections {
    pub fn new(
        attempts: Arc<Mutex<BootstrapAttempts>>,
        config: BootstrapInitiatorConfig,
        network: Arc<Network>,
        async_rt: Arc<AsyncRuntime>,
        workers: Arc<dyn ThreadPool>,
        socket_observer: Arc<dyn SocketObserver>,
        stats: Arc<Stats>,
        outbound_limiter: Arc<OutboundBandwidthLimiter>,
        block_processor: Arc<BlockProcessor>,
        pulls_cache: Arc<Mutex<PullsCache>>,
    ) -> Self {
        Self {
            condition: Condvar::new(),
            populate_connections_started: AtomicBool::new(false),
            attempts,
            mutex: Mutex::new(BootstrapConnectionsData {
                pulls: VecDeque::new(),
                clients: VecDeque::new(),
                idle: VecDeque::new(),
            }),
            config,
            connections_count: AtomicU32::new(0),
            new_connections_empty: AtomicBool::new(false),
            stopped: AtomicBool::new(false),
            network,
            workers,
            async_rt,
            socket_observer,
            stats,
            outbound_limiter,
            block_processor,
            pulls_cache,
            bootstrap_initiator: Mutex::new(None),
        }
    }

    pub fn new_null() -> Self {
        Self {
            condition: Condvar::new(),
            populate_connections_started: AtomicBool::new(false),
            attempts: Arc::new(Mutex::new(BootstrapAttempts::new())),
            mutex: Mutex::new(BootstrapConnectionsData::default()),
            config: BootstrapInitiatorConfig::default(),
            connections_count: AtomicU32::new(0),
            new_connections_empty: AtomicBool::new(false),
            stopped: AtomicBool::new(false),
            network: Arc::new(Network::new_null()),
            workers: Arc::new(ThreadPoolImpl::new_null()),
            async_rt: Arc::new(AsyncRuntime::default()),
            socket_observer: Arc::new(NullSocketObserver::new()),
            stats: Arc::new(Stats::default()),
            block_processor: Arc::new(BlockProcessor::new_null()),
            outbound_limiter: Arc::new(OutboundBandwidthLimiter::default()),
            bootstrap_initiator: Mutex::new(None),
            pulls_cache: Arc::new(Mutex::new(PullsCache::new())),
        }
    }

    pub fn set_bootstrap_initiator(&self, initiator: Arc<BootstrapInitiator>) {
        *self.bootstrap_initiator.lock().unwrap() = Some(Arc::downgrade(&initiator));
    }

    pub fn target_connections(&self, pulls_remaining: usize, attempts_count: usize) -> u32 {
        let attempts_factor = self.config.bootstrap_connections * attempts_count as u32;
        if attempts_factor >= self.config.bootstrap_connections_max {
            return max(1, self.config.bootstrap_connections_max);
        }

        // Only scale up to bootstrap_connections_max for large pulls.
        let step_scale = min(
            OrderedFloat(1f64),
            max(
                OrderedFloat(0f64),
                OrderedFloat(
                    pulls_remaining as f64
                        / bootstrap_limits::BOOTSTRAP_CONNECTION_SCALE_TARGET_BLOCKS as f64,
                ),
            ),
        );
        let target = attempts_factor as f64
            + (self.config.bootstrap_connections_max - attempts_factor) as f64 * step_scale.0;
        return max(1, (target + 0.5) as u32);
    }

    pub fn connection(&self, use_front_connection: bool) -> (Option<Arc<BootstrapClient>>, bool) {
        let mut guard = self.mutex.lock().unwrap();
        guard = self
            .condition
            .wait_while(guard, |i| {
                !self.stopped.load(Ordering::SeqCst)
                    && i.idle.is_empty()
                    && !self.new_connections_empty.load(Ordering::SeqCst)
            })
            .unwrap();

        let mut result = None;
        if !self.stopped.load(Ordering::SeqCst) && !guard.idle.is_empty() {
            if !use_front_connection {
                result = guard.idle.pop_back();
            } else {
                result = guard.idle.pop_front();
            }
        }
        if result.is_none()
            && self.connections_count.load(Ordering::SeqCst) == 0
            && self.new_connections_empty.load(Ordering::SeqCst)
        {
            (result, true) // should stop
        } else {
            (result, false) //don't stop
        }
    }

    pub fn bootstrap_client_closed(&self) {
        self.connections_count.fetch_sub(1, Ordering::SeqCst);
    }

    pub fn bootstrap_status(&self, tree: &mut dyn PropertyTree, attempts_count: usize) {
        let guard = self.mutex.lock().unwrap();
        tree.put_u64("clients", guard.clients.len() as u64).unwrap();
        tree.put_u64(
            "connections",
            self.connections_count.load(Ordering::SeqCst) as u64,
        )
        .unwrap();
        tree.put_u64("idle", guard.idle.len() as u64).unwrap();
        tree.put_u64(
            "target_connections",
            self.target_connections(guard.pulls.len(), attempts_count) as u64,
        )
        .unwrap();
        tree.put_u64("pulls", guard.pulls.len() as u64).unwrap();
    }

    pub fn clear_pulls(&self, bootstrap_id_a: u64) {
        {
            let mut guard = self.mutex.lock().unwrap();

            guard.pulls.retain(|i| i.bootstrap_id != bootstrap_id_a);
        }
        self.condition.notify_all();
    }

    pub fn stop(&self) {
        let lock = self.mutex.lock().unwrap();
        self.stopped.store(true, Ordering::SeqCst);
        drop(lock);
        self.condition.notify_all();
        let mut lock = self.mutex.lock().unwrap();
        for i in &lock.clients {
            if let Some(client) = i.upgrade() {
                client.close_socket();
            }
        }
        lock.clients.clear();
        lock.idle.clear();
    }
}

pub trait BootstrapConnectionsExt {
    fn pool_connection(&self, client: Arc<BootstrapClient>, new_client: bool, push_front: bool);
    fn requeue_pull(&self, pull: PullInfo, network_error: bool);
    fn run(&self);
    fn start_populate_connections(&self);
    fn populate_connections(&self, repeat: bool);
    fn add_pull(&self, pull: PullInfo);
    fn connection(&self, use_front_connection: bool) -> (Option<Arc<BootstrapClient>>, bool);
    fn find_connection(&self, endpoint: SocketAddrV6) -> Option<Arc<BootstrapClient>>;
    fn add_connection(&self, endpoint: SocketAddrV6);
    fn connect_client(&self, endpoint: SocketAddrV6, push_front: bool);
    fn request_pull<'a>(
        &'a self,
        guard: MutexGuard<'a, BootstrapConnectionsData>,
    ) -> MutexGuard<'a, BootstrapConnectionsData>;
}

impl BootstrapConnectionsExt for Arc<BootstrapConnections> {
    fn pool_connection(&self, client_a: Arc<BootstrapClient>, new_client: bool, push_front: bool) {
        let mut guard = self.mutex.lock().unwrap();
        if !self.stopped.load(Ordering::SeqCst)
            && !client_a.pending_stop()
            && !self.network.is_excluded(&client_a.tcp_endpoint())
        {
            client_a.set_timeout(self.config.idle_timeout);
            // Push into idle deque
            if !push_front {
                guard.idle.push_back(Arc::clone(&client_a));
            } else {
                guard.idle.push_front(Arc::clone(&client_a));
            }
            if new_client {
                guard.clients.push_back(Arc::downgrade(&client_a));
            }
        } else {
            client_a.close_socket();
        }
        drop(guard);
        self.condition.notify_all();
    }

    fn requeue_pull(&self, pull_a: PullInfo, network_error: bool) {
        let mut pull = pull_a;
        if !network_error {
            pull.attempts += 1;
        }
        let attempt_l = self
            .attempts
            .lock()
            .unwrap()
            .find(pull.bootstrap_id as usize)
            .cloned();
        if let Some(attempt_l) = attempt_l {
            attempt_l
                .attempt()
                .requeued_pulls
                .fetch_add(1, Ordering::SeqCst);
            let mut is_lazy = false;
            if let BootstrapStrategy::Lazy(lazy) = &*attempt_l {
                is_lazy = true;
                pull.count = lazy.lazy_batch_size();
            }
            if attempt_l.mode() == BootstrapMode::Legacy
                && (pull.attempts
                    < pull.retry_limit
                        + (pull.processed
                            / bootstrap_limits::REQUEUED_PULLS_PROCESSED_BLOCKS_FACTOR as u64)
                            as u32)
            {
                {
                    let mut guard = self.mutex.lock().unwrap();
                    guard.pulls.push_front(pull);
                }
                attempt_l.attempt().pull_started();
                self.condition.notify_all();
            } else if is_lazy
                && (pull.attempts
                    <= pull.retry_limit
                        + (pull.processed as u32 / self.config.lazy_max_pull_blocks))
            {
                debug_assert_eq!(BlockHash::from(pull.account_or_head), pull.head);

                let BootstrapStrategy::Lazy(lazy) = &*attempt_l else {
                    unreachable!()
                };
                if !lazy.lazy_processed_or_exists(&pull.account_or_head.into()) {
                    {
                        let mut guard = self.mutex.lock().unwrap();
                        guard.pulls.push_back(pull);
                    }
                    attempt_l.attempt().pull_started();
                    self.condition.notify_all();
                }
            } else {
                self.stats.inc_dir(
                    StatType::Bootstrap,
                    DetailType::BulkPullFailedAccount,
                    Direction::In,
                );
                debug!("Failed to pull account {} or head block {} down to {} after {} attempts and {} blocks processed",
                		Account::from(pull.account_or_head).encode_account(),
                		pull.account_or_head,
                		pull.end,
                		pull.attempts,
                		pull.processed);

                if is_lazy && pull.processed > 0 {
                    let BootstrapStrategy::Lazy(lazy) = &*attempt_l else {
                        unreachable!()
                    };
                    lazy.lazy_add(&pull);
                } else if attempt_l.mode() == BootstrapMode::Legacy {
                    self.pulls_cache.lock().unwrap().add(&pull);
                }
            }
        }
    }

    fn add_pull(&self, mut pull: PullInfo) {
        self.pulls_cache.lock().unwrap().update_pull(&mut pull);
        {
            let mut guard = self.mutex.lock().unwrap();
            guard.pulls.push_back(pull);
        }
        self.condition.notify_all();
    }

    fn connection(&self, use_front_connection: bool) -> (Option<Arc<BootstrapClient>>, bool) {
        let mut guard = self.mutex.lock().unwrap();
        guard = self
            .condition
            .wait_while(guard, |g| {
                !self.stopped.load(Ordering::SeqCst)
                    && g.idle.is_empty()
                    && !self.new_connections_empty.load(Ordering::SeqCst)
            })
            .unwrap();
        let mut result: Option<Arc<BootstrapClient>> = None;
        if !self.stopped.load(Ordering::SeqCst) && !guard.idle.is_empty() {
            if !use_front_connection {
                result = guard.idle.pop_back();
            } else {
                result = guard.idle.pop_front();
            }
        }
        if result.is_none()
            && self.connections_count.load(Ordering::SeqCst) == 0
            && self.new_connections_empty.load(Ordering::SeqCst)
        {
            (result, true) // should stop attempt
        } else {
            (result, false) // should not stop attempt
        }
    }

    fn run(&self) {
        self.start_populate_connections();
        let mut guard = self.mutex.lock().unwrap();
        while !self.stopped.load(Ordering::SeqCst) {
            if !guard.pulls.is_empty() {
                guard = self.request_pull(guard);
            } else {
                guard = self.condition.wait(guard).unwrap();
            }
        }
        self.stopped.store(true, Ordering::SeqCst);
        drop(guard);
        self.condition.notify_all();
    }

    fn start_populate_connections(&self) {
        if !self.populate_connections_started.load(Ordering::SeqCst) {
            self.populate_connections(true);
        }
    }

    fn find_connection(&self, endpoint: SocketAddrV6) -> Option<Arc<BootstrapClient>> {
        let mut guard = self.mutex.lock().unwrap();
        let mut result = None;
        for (i, client) in guard.idle.iter().enumerate() {
            if self.stopped.load(Ordering::SeqCst) {
                break;
            }
            if client.tcp_endpoint() == endpoint {
                result = Some(Arc::clone(client));
                guard.idle.remove(i);
                break;
            }
        }
        result
    }

    fn populate_connections(&self, repeat: bool) {
        let mut rate_sum = 0f64;
        let num_pulls;
        let attempts_count = self.attempts.lock().unwrap().size();
        let mut sorted_connections: BinaryHeap<OrderedByBlockRateDesc> = BinaryHeap::new();
        let mut endpoints = HashSet::new();
        {
            let mut guard = self.mutex.lock().unwrap();
            num_pulls = guard.pulls.len();
            let mut new_clients = VecDeque::new();
            for c in &guard.clients {
                if let Some(client) = c.upgrade() {
                    new_clients.push_back(Arc::downgrade(&client));
                    endpoints.insert(into_ipv6_socket_address(client.remote_endpoint()));
                    let elapsed = client.elapsed();
                    let blocks_per_sec = client.sample_block_rate();
                    rate_sum += blocks_per_sec;
                    if client.elapsed().as_secs_f64()
                        > bootstrap_limits::BOOTSTRAP_CONNECTION_WARMUP_TIME_SEC
                        && client.block_count() > 0
                    {
                        sorted_connections.push(OrderedByBlockRateDesc(Arc::clone(&client)));
                    }
                    // Force-stop the slowest peers, since they can take the whole bootstrap hostage by dribbling out blocks on the last remaining pull.
                    // This is ~1.5kilobits/sec.
                    if elapsed.as_secs_f64()
                        > bootstrap_limits::BOOTSTRAP_MINIMUM_TERMINATION_TIME_SEC
                        && blocks_per_sec < bootstrap_limits::BOOTSTRAP_MINIMUM_BLOCKS_PER_SEC
                    {
                        debug!("Stopping slow peer {} (elapsed sec {} > {} and {} blocks per second < {})",
                        				client.channel_string(),
                        				elapsed.as_secs_f64(),
                        				bootstrap_limits::BOOTSTRAP_MINIMUM_TERMINATION_TIME_SEC,
                        				blocks_per_sec,
                        				bootstrap_limits::BOOTSTRAP_MINIMUM_BLOCKS_PER_SEC);

                        client.stop(true);
                        new_clients.pop_back();
                    }
                }
            }
            // Cleanup expired clients
            std::mem::swap(&mut guard.clients, &mut new_clients);
        }

        let target = self.target_connections(num_pulls, attempts_count);

        // We only want to drop slow peers when more than 2/3 are active. 2/3 because 1/2 is too aggressive, and 100% rarely happens.
        // Probably needs more tuning.
        if sorted_connections.len() >= (target as usize * 2) / 3 && target >= 4 {
            // 4 -> 1, 8 -> 2, 16 -> 4, arbitrary, but seems to work well.
            let drop = (target as f32 - 2.0).sqrt().round() as i32;

            debug!(
                "Dropping {} bulk pull peers, target connections {}",
                drop, target
            );

            for _ in 0..drop {
                if let Some(client) = sorted_connections.pop() {
                    debug!(
                        "Dropping peer with block rate {} and block count {} ({})",
                        client.block_rate(),
                        client.block_count(),
                        client.channel_string()
                    );

                    client.stop(false);
                }
            }
        }

        debug!("Bulk pull connections: {}, rate: {} blocks/sec, bootstrap attempts {}, remaining pulls: {}",
            self.connections_count.load(Ordering::SeqCst),
            rate_sum as f32,
            attempts_count,
            num_pulls);

        if self.connections_count.load(Ordering::SeqCst) < target
            && (attempts_count != 0 || self.new_connections_empty.load(Ordering::SeqCst))
            && !self.stopped.load(Ordering::SeqCst)
        {
            let delta = min(
                (target - self.connections_count.load(Ordering::SeqCst)) * 2,
                bootstrap_limits::BOOTSTRAP_MAX_NEW_CONNECTIONS,
            );
            // TODO - tune this better
            // Not many peers respond, need to try to make more connections than we need.
            for _ in 0..delta {
                let endpoint = self.network.bootstrap_peer(); // Legacy bootstrap is compatible with older version of protocol
                if endpoint != SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 0, 0, 0)
                    && (self.config.allow_bootstrap_peers_duplicates
                        || !endpoints.contains(&endpoint))
                    && !self.network.is_excluded(&endpoint)
                {
                    self.connect_client(endpoint, false);
                    endpoints.insert(endpoint);
                    let _guard = self.mutex.lock().unwrap();
                    self.new_connections_empty.store(false, Ordering::SeqCst);
                } else if self.connections_count.load(Ordering::SeqCst) == 0 {
                    {
                        let _guard = self.mutex.lock().unwrap();
                        self.new_connections_empty.store(true, Ordering::SeqCst);
                    }
                    self.condition.notify_all();
                }
            }
        }
        if !self.stopped.load(Ordering::SeqCst) && repeat {
            let self_w = Arc::downgrade(self);
            self.workers.add_delayed_task(
                Duration::from_secs(1),
                Box::new(move || {
                    if let Some(self_l) = self_w.upgrade() {
                        self_l.populate_connections(true);
                    }
                }),
            );
        }
    }

    fn add_connection(&self, endpoint: SocketAddrV6) {
        self.connect_client(endpoint, true);
    }

    fn connect_client(&self, endpoint: SocketAddrV6, push_front: bool) {
        self.connections_count.fetch_add(1, Ordering::SeqCst);
        let socket = SocketBuilder::new(
            ChannelDirection::Outbound,
            Arc::clone(&self.workers),
            Arc::downgrade(&self.async_rt),
        )
        .default_timeout(self.config.tcp_io_timeout)
        .silent_connection_tolerance_time(self.config.silent_connection_tolerance_time)
        .idle_timeout(self.config.idle_timeout)
        .observer(Arc::clone(&self.socket_observer))
        .finish();

        let self_l = Arc::clone(self);
        let socket_l = Arc::clone(&socket);
        socket.async_connect(
            endpoint,
            Box::new(move |ec| {
                if ec.is_ok() {
                    debug!("Connection established to: {}", endpoint);

                    let channel_id = self_l.network.get_next_channel_id();

                    let protocol = self_l.config.protocol;
                    let tcp_channel = Arc::new(ChannelEnum::Tcp(Arc::new(ChannelTcp::new(
                        Arc::clone(&socket_l),
                        SystemTime::now(),
                        Arc::clone(&self_l.stats),
                        Arc::clone(&self_l.outbound_limiter),
                        &self_l.async_rt,
                        channel_id,
                        protocol,
                    ))));

                    let client = Arc::new(BootstrapClient::new(
                        Arc::clone(&self_l.async_rt),
                        &self_l,
                        tcp_channel,
                        socket_l,
                    ));

                    self_l.connections_count.fetch_add(1, Ordering::SeqCst);
                    self_l.pool_connection(client, true, push_front);
                } else {
                    debug!(
                        "Error initiating bootstrap connection to: {} ({:?})",
                        endpoint, ec
                    );
                }
                self_l.connections_count.fetch_sub(1, Ordering::SeqCst);
            }),
        );
    }

    fn request_pull<'a>(
        &'a self,
        mut guard: MutexGuard<'a, BootstrapConnectionsData>,
    ) -> MutexGuard<'a, BootstrapConnectionsData> {
        drop(guard);
        let (connection_l, _should_stop) = self.connection(false);
        guard = self.mutex.lock().unwrap();
        if let Some(connection_l) = connection_l {
            if !guard.pulls.is_empty() {
                let mut attempt_l = None;
                let mut pull = PullInfo::default();
                // Search pulls with existing attempts
                while attempt_l.is_none() && !guard.pulls.is_empty() {
                    pull = guard.pulls.pop_front().unwrap();
                    attempt_l = self
                        .attempts
                        .lock()
                        .unwrap()
                        .find(pull.bootstrap_id as usize)
                        .cloned();
                    // Check if lazy pull is obsolete (head was processed or head is 0 for destinations requests)
                    if let Some(attempt) = &attempt_l {
                        if let BootstrapStrategy::Lazy(lazy) = &**attempt {
                            if !pull.head.is_zero() && lazy.lazy_processed_or_exists(&pull.head) {
                                attempt.attempt().pull_finished();
                                attempt_l = None;
                            }
                        }
                    }
                }

                if let Some(attempt_l) = attempt_l {
                    // The bulk_pull_client destructor attempt to requeue_pull which can cause a deadlock if this is the last reference
                    // Dispatch request in an external thread in case it needs to be destroyed
                    let self_l = Arc::clone(self);
                    let initiator = self_l
                        .bootstrap_initiator
                        .lock()
                        .unwrap()
                        .as_ref()
                        .cloned()
                        .expect("bootstrap initiator not set")
                        .upgrade();

                    let client_config = BulkPullClientConfig {
                        disable_legacy_bootstrap: self.config.disable_legacy_bootstrap,
                        retry_limit: self.config.lazy_retry_limit,
                        work_thresholds: self.config.work_thresholds.clone(),
                    };

                    if let Some(initiator) = initiator {
                        self.workers.push_task(Box::new(move || {
                            let client = Arc::new(BulkPullClient::new(
                                client_config,
                                Arc::clone(&self_l.stats),
                                Arc::clone(&self_l.block_processor),
                                connection_l,
                                attempt_l,
                                Arc::clone(&self_l.workers),
                                Arc::clone(&self_l.async_rt),
                                self_l,
                                initiator,
                                pull,
                            ));
                            client.request();
                        }));
                    }
                }
            } else {
                // Reuse connection if pulls deque become empty
                drop(guard);
                self.pool_connection(connection_l, false, false);
                guard = self.mutex.lock().unwrap();
            }
        }

        guard
    }
}

#[derive(Default)]
pub struct BootstrapConnectionsData {
    pulls: VecDeque<PullInfo>,
    clients: VecDeque<Weak<BootstrapClient>>,
    idle: VecDeque<Arc<BootstrapClient>>,
}

struct OrderedByBlockRateDesc(Arc<BootstrapClient>);

impl Deref for OrderedByBlockRateDesc {
    type Target = Arc<BootstrapClient>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Ord for OrderedByBlockRateDesc {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        OrderedFloat(other.0.block_rate()).cmp(&OrderedFloat(self.0.block_rate()))
    }
}

impl PartialOrd for OrderedByBlockRateDesc {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(&other))
    }
}

impl PartialEq for OrderedByBlockRateDesc {
    fn eq(&self, other: &Self) -> bool {
        self.0.block_rate() == other.0.block_rate()
    }
}

impl Eq for OrderedByBlockRateDesc {}
