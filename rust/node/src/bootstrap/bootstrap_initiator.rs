use super::{
    BootstrapAttemptLazy, BootstrapAttemptLegacy, BootstrapAttempts, BootstrapConnections,
    BootstrapConnectionsExt, BootstrapMode, BootstrapStrategy, LegacyBootstrapConfig, PullInfo,
    PullsCache,
};
use crate::{
    block_processing::BlockProcessor,
    bootstrap::BootstrapAttemptWallet,
    config::NodeFlags,
    stats::{DetailType, Direction, StatType, Stats},
    transport::{Network, OutboundBandwidthLimiter, SocketObserver},
    utils::{AsyncRuntime, ThreadPool, ThreadPoolImpl},
    websocket::WebsocketListener,
    NetworkParams,
};
use rsnano_core::{
    utils::{ContainerInfo, ContainerInfoComponent},
    work::WorkThresholds,
    Account, Amount, HashOrAccount, Networks, XRB_RATIO,
};
use rsnano_ledger::Ledger;
use rsnano_messages::ProtocolInfo;
use std::{
    collections::{HashMap, VecDeque},
    net::SocketAddrV6,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Condvar, Mutex,
    },
    thread::JoinHandle,
    time::Duration,
};

#[derive(Clone)]
pub struct BootstrapInitiatorConfig {
    pub bootstrap_connections: u32,
    pub bootstrap_connections_max: u32,
    pub tcp_io_timeout: Duration,
    pub silent_connection_tolerance_time: Duration,
    pub allow_bootstrap_peers_duplicates: bool,
    pub disable_legacy_bootstrap: bool,
    /** Default maximum idle time for a socket before it's automatically closed */
    pub idle_timeout: Duration,
    pub lazy_max_pull_blocks: u32,
    pub work_thresholds: WorkThresholds,
    pub lazy_retry_limit: u32,
    pub protocol: ProtocolInfo,
    pub frontier_request_count: u32,
    pub frontier_retry_limit: u32,
    pub disable_bulk_push_client: bool,
    pub bootstrap_initiator_threads: u32,
    pub receive_minimum: Amount,
}

impl Default for BootstrapInitiatorConfig {
    fn default() -> Self {
        Self {
            bootstrap_connections: 4,
            bootstrap_connections_max: 64,
            tcp_io_timeout: Duration::from_secs(15),
            silent_connection_tolerance_time: Duration::from_secs(120),
            allow_bootstrap_peers_duplicates: false,
            disable_legacy_bootstrap: false,
            idle_timeout: Duration::from_secs(120),
            lazy_max_pull_blocks: 512,
            work_thresholds: Default::default(),
            lazy_retry_limit: 64,
            protocol: Default::default(),
            frontier_request_count: 1024 * 1024,
            frontier_retry_limit: 16,
            disable_bulk_push_client: false,
            bootstrap_initiator_threads: 1,
            receive_minimum: Amount::raw(*XRB_RATIO),
        }
    }
}

impl From<&BootstrapInitiatorConfig> for LegacyBootstrapConfig {
    fn from(value: &BootstrapInitiatorConfig) -> Self {
        Self {
            frontier_request_count: value.frontier_request_count,
            frontier_retry_limit: value.frontier_retry_limit,
            disable_bulk_push_client: value.disable_bulk_push_client,
        }
    }
}

pub struct BootstrapInitiator {
    mutex: Mutex<Data>,
    condition: Condvar,
    threads: Mutex<Vec<JoinHandle<()>>>,
    pub connections: Arc<BootstrapConnections>,
    config: BootstrapInitiatorConfig,
    stopped: AtomicBool,
    pub cache: Arc<Mutex<PullsCache>>,
    stats: Arc<Stats>,
    pub attempts: Arc<Mutex<BootstrapAttempts>>,
    websocket: Option<Arc<WebsocketListener>>,
    block_processor: Arc<BlockProcessor>,
    ledger: Arc<Ledger>,
    network_params: NetworkParams,
    flags: NodeFlags,
    network: Arc<Network>,
    workers: Arc<dyn ThreadPool>,
}

impl BootstrapInitiator {
    pub fn new(
        config: BootstrapInitiatorConfig,
        flags: NodeFlags,
        network: Arc<Network>,
        async_rt: Arc<AsyncRuntime>,
        workers: Arc<dyn ThreadPool>,
        network_params: NetworkParams,
        socket_observer: Arc<dyn SocketObserver>,
        stats: Arc<Stats>,
        outbound_limiter: Arc<OutboundBandwidthLimiter>,
        block_processor: Arc<BlockProcessor>,
        websocket: Option<Arc<WebsocketListener>>,
        ledger: Arc<Ledger>,
    ) -> Self {
        let attempts = Arc::new(Mutex::new(BootstrapAttempts::new()));
        let cache = Arc::new(Mutex::new(PullsCache::new()));
        Self {
            mutex: Mutex::new(Data {
                attempts_list: HashMap::new(),
            }),
            condition: Condvar::new(),
            threads: Mutex::new(Vec::new()),
            config: config.clone(),
            stopped: AtomicBool::new(false),
            cache: Arc::clone(&cache),
            stats: Arc::clone(&stats),
            attempts: Arc::clone(&attempts),
            websocket,
            block_processor: Arc::clone(&block_processor),
            ledger,
            network_params: network_params.clone(),
            flags: flags.clone(),
            network: Arc::clone(&network),
            workers: Arc::clone(&workers),
            connections: Arc::new(BootstrapConnections::new(
                attempts,
                config,
                network,
                async_rt,
                workers,
                socket_observer,
                stats,
                outbound_limiter,
                block_processor,
                cache,
            )),
        }
    }

    pub fn new_null() -> Self {
        Self {
            mutex: Mutex::new(Data {
                attempts_list: HashMap::new(),
            }),
            condition: Condvar::new(),
            threads: Mutex::new(Vec::new()),
            connections: Arc::new(BootstrapConnections::new_null()),
            config: Default::default(),
            stopped: AtomicBool::new(false),
            cache: Arc::new(Mutex::new(PullsCache::new())),
            stats: Arc::new(Stats::default()),
            attempts: Arc::new(Mutex::new(BootstrapAttempts::new())),
            websocket: None,
            block_processor: Arc::new(BlockProcessor::new_null()),
            ledger: Arc::new(Ledger::new_null()),
            network_params: NetworkParams::new(Networks::NanoDevNetwork),
            flags: NodeFlags::default(),
            network: Arc::new(Network::new_null()),
            workers: Arc::new(ThreadPoolImpl::new_test_instance()),
        }
    }

    fn run_bootstrap(&self) {
        let mut guard = self.mutex.lock().unwrap();
        while !self.stopped.load(Ordering::SeqCst) {
            if guard.has_new_attempts() {
                let attempt = guard.new_attempt();
                drop(guard);
                if let Some(attempt) = attempt {
                    attempt.run();
                    self.remove_attempt(attempt);
                }
                guard = self.mutex.lock().unwrap();
            } else {
                guard = self.condition.wait(guard).unwrap();
            }
        }
    }

    pub fn clear_pulls(&self, bootstrap_id: u64) {
        self.connections.clear_pulls(bootstrap_id);
    }

    pub fn in_progress(&self) -> bool {
        !self.mutex.lock().unwrap().attempts_list.is_empty()
    }

    fn remove_attempt(&self, attempt_a: Arc<BootstrapStrategy>) {
        let mut guard = self.mutex.lock().unwrap();
        let incremental_id = attempt_a.incremental_id() as usize;
        let attempt = guard.attempts_list.get(&incremental_id).cloned();
        if let Some(attempt) = attempt {
            self.attempts.lock().unwrap().remove(incremental_id);
            guard.attempts_list.remove(&incremental_id);
            debug_assert_eq!(
                self.attempts.lock().unwrap().size(),
                guard.attempts_list.len()
            );
            drop(guard);
            attempt.stop();
        } else {
            drop(guard);
        }
        self.condition.notify_all();
    }

    pub fn current_legacy_attempt(&self) -> Option<Arc<BootstrapStrategy>> {
        let guard = self.mutex.lock().unwrap();
        guard.find_attempt(BootstrapMode::Legacy)
    }

    pub fn current_lazy_attempt(&self) -> Option<Arc<BootstrapStrategy>> {
        let guard = self.mutex.lock().unwrap();
        guard.find_attempt(BootstrapMode::Lazy)
    }

    pub fn current_wallet_attempt(&self) -> Option<Arc<BootstrapStrategy>> {
        let guard = self.mutex.lock().unwrap();
        guard.find_attempt(BootstrapMode::WalletLazy)
    }

    fn stop_attempts(&self) {
        let mut guard = self.mutex.lock().unwrap();
        let mut copy_attempts = HashMap::new();
        std::mem::swap(&mut copy_attempts, &mut guard.attempts_list);
        self.attempts.lock().unwrap().clear();
        drop(guard);
        for i in copy_attempts.values() {
            i.stop();
        }
    }

    pub fn remove_from_cache(&self, pull: &PullInfo) {
        self.cache.lock().unwrap().remove(pull);
    }

    pub fn collect_container_info(&self, name: impl Into<String>) -> ContainerInfoComponent {
        let cache_count = self.cache.lock().unwrap().size();
        ContainerInfoComponent::Composite(
            name.into(),
            vec![ContainerInfoComponent::Leaf(ContainerInfo {
                name: "pulls_cache".to_string(),
                count: cache_count,
                sizeof_element: PullsCache::ELEMENT_SIZE,
            })],
        )
    }
}

impl Drop for BootstrapInitiator {
    fn drop(&mut self) {
        assert_eq!(0, self.threads.lock().unwrap().len());
    }
}

pub trait BootstrapInitiatorExt {
    fn initialize(&self);
    fn start(&self);
    fn stop(&self);
    fn bootstrap(&self, force: bool, id_a: String, frontiers_age_a: u32, start_account_a: Account);
    fn bootstrap2(&self, endpoint_a: SocketAddrV6, id_a: String);
    fn bootstrap_lazy(&self, hash_or_account_a: HashOrAccount, force: bool, id_a: String) -> bool;
    fn bootstrap_wallet(&self, accounts_a: VecDeque<Account>);
}

impl BootstrapInitiatorExt for Arc<BootstrapInitiator> {
    fn initialize(&self) {
        self.connections.set_bootstrap_initiator(Arc::clone(self));
    }

    fn start(&self) {
        let mut threads = self.threads.lock().unwrap();
        let conns = Arc::clone(&self.connections);
        threads.push(
            std::thread::Builder::new()
                .name("Bootstrap conn".to_string())
                .spawn(move || {
                    conns.run();
                })
                .unwrap(),
        );

        for _ in 0..self.config.bootstrap_initiator_threads {
            let self_l = Arc::clone(self);
            threads.push(
                std::thread::Builder::new()
                    .name("Bootstrap init".to_string())
                    .spawn(move || {
                        self_l.run_bootstrap();
                    })
                    .unwrap(),
            );
        }
    }

    fn stop(&self) {
        if !self.stopped.swap(true, Ordering::SeqCst) {
            self.stop_attempts();
            self.connections.stop();
            self.condition.notify_all();

            let mut threads = self.threads.lock().unwrap();
            for thread in threads.drain(..) {
                thread.join().unwrap();
            }
        }
    }

    fn bootstrap(&self, force: bool, id_a: String, frontiers_age_a: u32, start_account_a: Account) {
        if force {
            self.stop_attempts();
        }
        let mut guard = self.mutex.lock().unwrap();
        if !self.stopped.load(Ordering::SeqCst)
            && guard.find_attempt(BootstrapMode::Legacy).is_none()
        {
            self.stats.inc_dir(
                StatType::Bootstrap,
                if frontiers_age_a == u32::MAX {
                    DetailType::Initiate
                } else {
                    DetailType::InitiateLegacyAge
                },
                Direction::Out,
            );
            let incremental_id = self.attempts.lock().unwrap().get_incremental_id();
            let self_w = Arc::downgrade(self);
            let legacy_attempt = Arc::new(
                BootstrapAttemptLegacy::new(
                    self.websocket.as_ref().cloned(),
                    Arc::downgrade(&self.block_processor),
                    self_w,
                    Arc::clone(&self.ledger),
                    self.workers.clone(),
                    id_a,
                    incremental_id as u64,
                    Arc::clone(&self.connections),
                    (&self.config).into(),
                    Arc::clone(&self.stats),
                    frontiers_age_a,
                    start_account_a,
                )
                .unwrap(),
            );

            let attempt = Arc::new(BootstrapStrategy::Legacy(legacy_attempt));
            guard
                .attempts_list
                .insert(incremental_id, Arc::clone(&attempt));
            self.attempts.lock().unwrap().add(attempt);
            drop(guard);
            self.condition.notify_all();
        }
    }

    fn bootstrap2(&self, endpoint_a: SocketAddrV6, id_a: String) {
        if !self.stopped.load(Ordering::SeqCst) {
            self.stop_attempts();
            self.stats
                .inc_dir(StatType::Bootstrap, DetailType::Initiate, Direction::Out);
            let mut guard = self.mutex.lock().unwrap();
            let self_w = Arc::downgrade(self);
            let incremental_id = self.attempts.lock().unwrap().get_incremental_id();
            let legacy_attempt = Arc::new(
                BootstrapAttemptLegacy::new(
                    self.websocket.as_ref().cloned(),
                    Arc::downgrade(&self.block_processor),
                    self_w,
                    self.ledger.clone(),
                    self.workers.clone(),
                    id_a,
                    incremental_id as u64,
                    self.connections.clone(),
                    (&self.config).into(),
                    self.stats.clone(),
                    u32::MAX,
                    Account::zero(),
                )
                .unwrap(),
            );
            let attempt = Arc::new(BootstrapStrategy::Legacy(legacy_attempt));
            guard
                .attempts_list
                .insert(incremental_id, Arc::clone(&attempt));
            self.attempts.lock().unwrap().add(attempt);
            if !self.network.is_excluded(&endpoint_a) {
                self.connections.add_connection(endpoint_a);
            }
        }
        self.condition.notify_all();
    }

    fn bootstrap_lazy(&self, hash_or_account_a: HashOrAccount, force: bool, id_a: String) -> bool {
        let mut key_inserted = false;
        let lazy_attempt = self.current_lazy_attempt();
        if lazy_attempt.is_none() || force {
            if force {
                self.stop_attempts();
            }
            self.stats.inc_dir(
                StatType::Bootstrap,
                DetailType::InitiateLazy,
                Direction::Out,
            );
            let mut guard = self.mutex.lock().unwrap();
            if !self.stopped.load(Ordering::SeqCst)
                && guard.find_attempt(BootstrapMode::Lazy).is_none()
            {
                let incremental_id = self.attempts.lock().unwrap().get_incremental_id();
                let lazy_attempt = BootstrapAttemptLazy::new(
                    self.websocket.clone(),
                    Arc::clone(&self.block_processor),
                    Arc::downgrade(self),
                    Arc::clone(&self.ledger),
                    if id_a.is_empty() {
                        hash_or_account_a.to_string()
                    } else {
                        id_a
                    },
                    incremental_id as u64,
                    self.flags.clone(),
                    Arc::clone(&self.connections),
                    self.network_params.clone(),
                )
                .unwrap();
                let attempt = Arc::new(BootstrapStrategy::Lazy(lazy_attempt));
                guard
                    .attempts_list
                    .insert(incremental_id, Arc::clone(&attempt));
                self.attempts.lock().unwrap().add(Arc::clone(&attempt));

                let BootstrapStrategy::Lazy(lazy) = &*attempt else {
                    unreachable!()
                };
                key_inserted = lazy.lazy_start(&hash_or_account_a);
            }
        } else {
            let lazy_attempt = lazy_attempt.unwrap();
            let BootstrapStrategy::Lazy(lazy) = &*lazy_attempt else {
                unreachable!()
            };
            key_inserted = lazy.lazy_start(&hash_or_account_a);
        }
        self.condition.notify_all();
        key_inserted
    }

    fn bootstrap_wallet(&self, mut accounts_a: VecDeque<Account>) {
        debug_assert!(!accounts_a.is_empty());
        let wallet_attempt = self.current_wallet_attempt();
        self.stats.inc_dir(
            StatType::Bootstrap,
            DetailType::InitiateWalletLazy,
            Direction::Out,
        );
        if wallet_attempt.is_none() {
            let mut guard = self.mutex.lock().unwrap();
            let id = if !accounts_a.is_empty() {
                accounts_a[0].encode_account()
            } else {
                "".to_string()
            };
            let incremental_id = self.attempts.lock().unwrap().get_incremental_id();
            let wallet_attempt = Arc::new(
                BootstrapAttemptWallet::new(
                    self.websocket.clone(),
                    Arc::clone(&self.block_processor),
                    Arc::clone(self),
                    Arc::clone(&self.ledger),
                    id,
                    incremental_id as u64,
                    Arc::clone(&self.connections),
                    Arc::clone(&self.workers),
                    self.config.receive_minimum,
                    Arc::clone(&self.stats),
                )
                .unwrap(),
            );
            let attempt = Arc::new(BootstrapStrategy::Wallet(Arc::clone(&wallet_attempt)));
            guard
                .attempts_list
                .insert(incremental_id, Arc::clone(&attempt));
            self.attempts.lock().unwrap().add(attempt);
            wallet_attempt.wallet_start(&mut accounts_a);
        } else {
            let wallet_attempt = wallet_attempt.unwrap();
            let BootstrapStrategy::Wallet(wallet) = &*wallet_attempt else {
                unreachable!()
            };
            wallet.wallet_start(&mut accounts_a);
        }
        self.condition.notify_all();
    }
}

struct Data {
    attempts_list: HashMap<usize, Arc<BootstrapStrategy>>,
}

impl Data {
    fn find_attempt(&self, mode_a: BootstrapMode) -> Option<Arc<BootstrapStrategy>> {
        for i in self.attempts_list.values() {
            if i.mode() == mode_a {
                return Some(Arc::clone(i));
            }
        }
        None
    }

    fn new_attempt(&self) -> Option<Arc<BootstrapStrategy>> {
        for i in self.attempts_list.values() {
            if !i.attempt().started.swap(true, Ordering::SeqCst) {
                return Some(Arc::clone(i));
            }
        }
        None
    }

    fn has_new_attempts(&self) -> bool {
        for i in self.attempts_list.values() {
            if !i.started() {
                return true;
            }
        }
        false
    }
}
