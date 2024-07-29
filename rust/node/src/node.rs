use crate::block_processing::BlockProcessorConfig;
use crate::bootstrap::{BootstrapAscendingConfig, BootstrapServerConfig};
use crate::consensus::{
    ActiveElectionsConfig, HintedSchedulerConfig, OptimisticSchedulerConfig, PriorityBucketConfig,
    RequestAggregatorConfig, VoteCacheConfig, VoteProcessorConfig,
};
use crate::monitor::MonitorConfig;
use crate::stats::StatsConfig;
use crate::transport::MessageProcessorConfig;
use crate::utils::TxnTrackingConfig;
use crate::websocket::WebsocketConfig;
use crate::IpcConfig;
use crate::{
    block_processing::LocalBlockBroadcasterConfig, bootstrap::BootstrapInitiatorConfig,
    cementation::ConfirmingSetConfig, transport::TcpConfig, NetworkParams, DEV_NETWORK_PARAMS,
};
use crate::{
    block_processing::{
        BacklogPopulation, BlockProcessor, BlockSource, LocalBlockBroadcaster,
        LocalBlockBroadcasterExt, UncheckedMap,
    },
    bootstrap::{
        BootstrapAscending, BootstrapAscendingExt, BootstrapInitiator, BootstrapInitiatorExt,
        BootstrapServer, OngoingBootstrap, OngoingBootstrapExt,
    },
    cementation::ConfirmingSet,
    config::{GlobalConfig, NodeFlags},
    consensus::{
        create_loopback_channel, get_bootstrap_weights, log_bootstrap_weights,
        AccountBalanceChangedCallback, ActiveElections, ActiveElectionsExt, ElectionEndCallback,
        ElectionStatusType, HintedScheduler, HintedSchedulerExt, LocalVoteHistory, ManualScheduler,
        ManualSchedulerExt, OptimisticScheduler, OptimisticSchedulerExt, PriorityScheduler,
        PrioritySchedulerExt, ProcessLiveDispatcher, ProcessLiveDispatcherExt,
        RecentlyConfirmedCache, RepTiers, RequestAggregator, RequestAggregatorExt, VoteApplier,
        VoteBroadcaster, VoteCache, VoteCacheProcessor, VoteGenerators, VoteProcessor,
        VoteProcessorExt, VoteProcessorQueue, VoteRouter,
    },
    monitor::Monitor,
    node_id_key_file::NodeIdKeyFile,
    pruning::{LedgerPruning, LedgerPruningExt},
    representatives::{OnlineReps, RepCrawler, RepCrawlerExt},
    stats::{DetailType, Direction, LedgerStats, StatType, Stats},
    transport::{
        BufferDropPolicy, ChannelEnum, InboundCallback, InboundMessageQueue, KeepaliveFactory,
        MessageProcessor, Network, NetworkFilter, NetworkOptions, NetworkThreads,
        OutboundBandwidthLimiter, PeerCacheConnector, PeerCacheUpdater, PeerConnector,
        RealtimeMessageHandler, ResponseServerFactory, SocketObserver, SynCookies, TcpListener,
        TcpListenerExt, TrafficType,
    },
    utils::{AsyncRuntime, LongRunningTransactionLogger, ThreadPool, ThreadPoolImpl, TimerThread},
    wallets::{Wallets, WalletsExt},
    websocket::{create_websocket_server, WebsocketListenerExt},
    work::{DistributedWorkFactory, HttpClient},
    OnlineWeightSampler, TelementryConfig, TelementryExt, Telemetry, BUILD_INFO, VERSION_STRING,
};
use anyhow::Result;
use once_cell::sync::Lazy;
use rand::{thread_rng, Rng};
use reqwest::Url;
use rsnano_core::{
    utils::{
        as_nano_json, system_time_as_nanoseconds, ContainerInfoComponent, SerdePropertyTree,
        SystemTimeFactory,
    },
    work::{WorkPool, WorkPoolImpl},
    Account, Amount, BlockEnum, BlockHash, BlockType, KeyPair, PublicKey, Root, Vote, VoteCode,
    VoteSource,
};
use rsnano_core::{
    utils::{get_env_or_default_string, is_sanitizer_build},
    Networks, GXRB_RATIO, XRB_RATIO,
};
use rsnano_ledger::{BlockStatus, Ledger, RepWeightCache};
use rsnano_messages::{ConfirmAck, DeserializedMessage, Message};
use rsnano_store_lmdb::LmdbConfig;
use rsnano_store_lmdb::{
    EnvOptions, LmdbEnv, LmdbStore, NullTransactionTracker, SyncStrategy, TransactionTracker,
};
use serde::Serialize;
use serde::{Deserialize, Deserializer, Serializer};
use std::fmt;
use std::str::FromStr;
use std::{
    borrow::Borrow,
    collections::{HashMap, VecDeque},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, RwLock,
    },
    time::{Duration, Instant, SystemTime},
};
use std::{cmp::max, net::Ipv6Addr};
use tracing::{debug, error, info, warn};

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, FromPrimitive, Deserialize, Serialize)]
pub enum FrontiersConfirmationMode {
    Always,    // Always confirm frontiers
    Automatic, // Always mode if node contains representative with at least 50% of principal weight, less frequest requests if not
    Disabled,  // Do not confirm frontiers
    Invalid,
}

#[derive(Clone)]
pub struct NodeConfig {
    pub peering_port: Option<u16>,
    pub optimistic_scheduler: OptimisticSchedulerConfig,
    pub hinted_scheduler: HintedSchedulerConfig,
    pub priority_bucket: PriorityBucketConfig,
    pub bootstrap_fraction_numerator: u32,
    pub receive_minimum: Amount,
    pub online_weight_minimum: Amount,
    /// The minimum vote weight that a representative must have for its vote to be counted.
    /// All representatives above this weight will be kept in memory!
    pub representative_vote_weight_minimum: Amount,
    pub password_fanout: u32,
    pub io_threads: u32,
    pub network_threads: u32,
    pub work_threads: u32,
    pub background_threads: u32,
    pub signature_checker_threads: u32,
    pub enable_voting: bool,
    pub bootstrap_connections: u32,
    pub bootstrap_connections_max: u32,
    pub bootstrap_initiator_threads: u32,
    pub bootstrap_serving_threads: u32,
    pub bootstrap_frontier_request_count: u32,
    pub block_processor_batch_max_time_ms: i64,
    pub allow_local_peers: bool,
    pub vote_minimum: Amount,
    pub vote_generator_delay_ms: i64,
    pub vote_generator_threshold: u32,
    pub unchecked_cutoff_time_s: i64,
    pub tcp_io_timeout_s: i64,
    pub pow_sleep_interval_ns: i64,
    pub external_address: String,
    pub external_port: u16,
    pub tcp_incoming_connections_max: u32,
    pub use_memory_pools: bool,
    pub bandwidth_limit: usize,
    pub bandwidth_limit_burst_ratio: f64,
    pub bootstrap_ascending: BootstrapAscendingConfig,
    pub bootstrap_server: BootstrapServerConfig,
    pub bootstrap_bandwidth_limit: usize,
    pub bootstrap_bandwidth_burst_ratio: f64,
    pub confirming_set_batch_time: Duration,
    pub backup_before_upgrade: bool,
    pub max_work_generate_multiplier: f64,
    pub frontiers_confirmation: FrontiersConfirmationMode,
    pub max_queued_requests: u32,
    pub request_aggregator_threads: u32,
    pub max_unchecked_blocks: u32,
    pub rep_crawler_weight_minimum: Amount,
    pub work_peers: Vec<Peer>,
    pub secondary_work_peers: Vec<Peer>,
    pub preconfigured_peers: Vec<String>,
    pub preconfigured_representatives: Vec<Account>,
    pub max_pruning_age_s: i64,
    pub max_pruning_depth: u64,
    pub callback_address: String,
    pub callback_port: u16,
    pub callback_target: String,
    pub websocket_config: WebsocketConfig,
    pub ipc_config: IpcConfig,
    pub txn_tracking_config: TxnTrackingConfig,
    pub stat_config: StatsConfig,
    pub lmdb_config: LmdbConfig,
    /// Number of accounts per second to process when doing backlog population scan
    pub backlog_scan_batch_size: u32,
    /// Number of times per second to run backlog population batches. Number of accounts per single batch is `backlog_scan_batch_size / backlog_scan_frequency`
    pub backlog_scan_frequency: u32,
    pub vote_cache: VoteCacheConfig,
    pub rep_crawler_query_timeout: Duration,
    pub block_processor: BlockProcessorConfig,
    pub active_elections: ActiveElectionsConfig,
    pub vote_processor: VoteProcessorConfig,
    pub tcp: TcpConfig,
    pub request_aggregator: RequestAggregatorConfig,
    pub message_processor: MessageProcessorConfig,
    pub priority_scheduler_enabled: bool,
    pub local_block_broadcaster: LocalBlockBroadcasterConfig,
    pub confirming_set: ConfirmingSetConfig,
    pub monitor: MonitorConfig,
}

#[derive(Clone)]
pub struct Peer {
    pub address: String,
    pub port: u16,
}

impl fmt::Display for Peer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.address, self.port)
    }
}

impl FromStr for Peer {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return Err("Invalid format".into());
        }

        let address = parts[0].to_string();
        let port = parts[1]
            .parse::<u16>()
            .map_err(|_| "Invalid port".to_string())?;

        Ok(Peer { address, port })
    }
}

impl Serialize for Peer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Peer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse::<Peer>().map_err(serde::de::Error::custom)
    }
}

impl Peer {
    pub fn new(address: impl Into<String>, port: u16) -> Self {
        Self {
            address: address.into(),
            port,
        }
    }
}

static DEFAULT_LIVE_PEER_NETWORK: Lazy<String> =
    Lazy::new(|| get_env_or_default_string("NANO_DEFAULT_PEER", "peering.nano.org"));

static DEFAULT_BETA_PEER_NETWORK: Lazy<String> =
    Lazy::new(|| get_env_or_default_string("NANO_DEFAULT_PEER", "peering-beta.nano.org"));

static DEFAULT_TEST_PEER_NETWORK: Lazy<String> =
    Lazy::new(|| get_env_or_default_string("NANO_DEFAULT_PEER", "peering-test.nano.org"));

impl NodeConfig {
    pub fn new(
        peering_port: Option<u16>,
        network_params: &NetworkParams,
        parallelism: usize,
    ) -> Self {
        if peering_port == Some(0) {
            // comment for posterity:
            // - we used to consider ports being 0 a sentinel that meant to use a default port for that specific purpose
            // - the actual default value was determined based on the active network (e.g. dev network peering port = 44000)
            // - now, the 0 value means something different instead: user wants to let the OS pick a random port
            // - for the specific case of the peering port, after it gets picked, it can be retrieved by client code via
            //   node.network.endpoint ().port ()
            // - the config value does not get back-propagated because it represents the choice of the user, and that was 0
        }

        let mut enable_voting = false;
        let mut preconfigured_peers = Vec::new();
        let mut preconfigured_representatives = Vec::new();
        match network_params.network.current_network {
            Networks::NanoDevNetwork => {
                enable_voting = true;
                preconfigured_representatives.push(network_params.ledger.genesis_account);
            }
            Networks::NanoBetaNetwork => {
                preconfigured_peers.push(DEFAULT_BETA_PEER_NETWORK.clone());
                preconfigured_representatives.push(
                    Account::decode_account(
                        "nano_1defau1t9off1ine9rep99999999999999999999999999999999wgmuzxxy",
                    )
                    .unwrap(),
                );
            }
            Networks::NanoLiveNetwork => {
                preconfigured_peers.push(DEFAULT_LIVE_PEER_NETWORK.clone());
                preconfigured_representatives.push(
                    Account::decode_hex(
                        "A30E0A32ED41C8607AA9212843392E853FCBCB4E7CB194E35C94F07F91DE59EF",
                    )
                    .unwrap(),
                );
                preconfigured_representatives.push(
                    Account::decode_hex(
                        "67556D31DDFC2A440BF6147501449B4CB9572278D034EE686A6BEE29851681DF",
                    )
                    .unwrap(),
                );
                preconfigured_representatives.push(
                    Account::decode_hex(
                        "5C2FBB148E006A8E8BA7A75DD86C9FE00C83F5FFDBFD76EAA09531071436B6AF",
                    )
                    .unwrap(),
                );
                preconfigured_representatives.push(
                    Account::decode_hex(
                        "AE7AC63990DAAAF2A69BF11C913B928844BF5012355456F2F164166464024B29",
                    )
                    .unwrap(),
                );
                preconfigured_representatives.push(
                    Account::decode_hex(
                        "BD6267D6ECD8038327D2BCC0850BDF8F56EC0414912207E81BCF90DFAC8A4AAA",
                    )
                    .unwrap(),
                );
                preconfigured_representatives.push(
                    Account::decode_hex(
                        "2399A083C600AA0572F5E36247D978FCFC840405F8D4B6D33161C0066A55F431",
                    )
                    .unwrap(),
                );
                preconfigured_representatives.push(
                    Account::decode_hex(
                        "2298FAB7C61058E77EA554CB93EDEEDA0692CBFCC540AB213B2836B29029E23A",
                    )
                    .unwrap(),
                );
                preconfigured_representatives.push(
                    Account::decode_hex(
                        "3FE80B4BC842E82C1C18ABFEEC47EA989E63953BC82AC411F304D13833D52A56",
                    )
                    .unwrap(),
                );
            }
            Networks::NanoTestNetwork => {
                preconfigured_peers.push(DEFAULT_TEST_PEER_NETWORK.clone());
                preconfigured_representatives.push(network_params.ledger.genesis_account);
            }
            Networks::Invalid => panic!("invalid network"),
        }

        Self {
            peering_port,
            bootstrap_fraction_numerator: 1,
            receive_minimum: Amount::raw(*XRB_RATIO),
            online_weight_minimum: Amount::nano(60_000_000),
            representative_vote_weight_minimum: Amount::nano(10),
            password_fanout: 1024,
            io_threads: max(parallelism, 4) as u32,
            network_threads: max(parallelism, 4) as u32,
            work_threads: max(parallelism, 4) as u32,
            background_threads: max(parallelism, 4) as u32,
            /* Use half available threads on the system for signature checking. The calling thread does checks as well, so these are extra worker threads */
            signature_checker_threads: (parallelism / 2) as u32,
            enable_voting,
            bootstrap_connections: BootstrapInitiatorConfig::default().bootstrap_connections,
            bootstrap_connections_max: BootstrapInitiatorConfig::default()
                .bootstrap_connections_max,
            bootstrap_initiator_threads: 1,
            bootstrap_serving_threads: 1,
            bootstrap_frontier_request_count: BootstrapInitiatorConfig::default()
                .frontier_request_count,
            block_processor_batch_max_time_ms: BlockProcessorConfig::default()
                .batch_max_time
                .as_millis() as i64,
            allow_local_peers: !(network_params.network.is_live_network()
                || network_params.network.is_test_network()), // disable by default for live network
            vote_minimum: Amount::raw(*GXRB_RATIO),
            vote_generator_delay_ms: 100,
            vote_generator_threshold: 3,
            unchecked_cutoff_time_s: 4 * 60 * 60, // 4 hours
            tcp_io_timeout_s: if network_params.network.is_dev_network() && !is_sanitizer_build() {
                5
            } else {
                15
            },
            pow_sleep_interval_ns: 0,
            external_address: Ipv6Addr::UNSPECIFIED.to_string(),
            external_port: 0,
            // Default maximum incoming TCP connections, including realtime network & bootstrap
            tcp_incoming_connections_max: 2048,
            use_memory_pools: true,
            // Default outbound traffic shaping is 10MB/s
            bandwidth_limit: 10 * 1024 * 1024,
            // By default, allow bursts of 15MB/s (not sustainable)
            bandwidth_limit_burst_ratio: 3_f64,
            // Default boostrap outbound traffic limit is 5MB/s
            bootstrap_bandwidth_limit: 5 * 1024 * 1024,
            // Bootstrap traffic does not need bursts
            bootstrap_bandwidth_burst_ratio: 1.,
            bootstrap_ascending: Default::default(),
            bootstrap_server: Default::default(),
            confirming_set_batch_time: Duration::from_millis(250),
            backup_before_upgrade: false,
            max_work_generate_multiplier: 64_f64,
            frontiers_confirmation: FrontiersConfirmationMode::Automatic,
            max_queued_requests: 512,
            request_aggregator_threads: max(parallelism, 4) as u32,
            max_unchecked_blocks: 65536,
            rep_crawler_weight_minimum: Amount::decode_hex("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF")
                .unwrap(),
            work_peers: Vec::new(),
            secondary_work_peers: vec![Peer::new("127.0.0.1", 8076)],
            preconfigured_peers,
            preconfigured_representatives,
            max_pruning_age_s: if !network_params.network.is_beta_network() {
                24 * 60 * 60
            } else {
                5 * 60
            }, // 1 day; 5 minutes for beta network
            max_pruning_depth: 0,
            callback_address: String::new(),
            callback_port: 0,
            callback_target: String::new(),
            websocket_config: WebsocketConfig::new(&network_params.network),
            ipc_config: IpcConfig::new(&network_params.network),
            txn_tracking_config: DiagnosticsConfig::new(),
            stat_config: StatsConfig::new(),
            lmdb_config: LmdbConfig::new(),
            backlog_scan_batch_size: 10 * 1000,
            backlog_scan_frequency: 10,
            optimistic_scheduler: OptimisticSchedulerConfig::new(),
            hinted_scheduler: if network_params.network.is_dev_network() {
                HintedSchedulerConfig::default_for_dev_network()
            } else {
                HintedSchedulerConfig::default()
            },
            priority_bucket: Default::default(),
            vote_cache: Default::default(),
            active_elections: Default::default(),
            rep_crawler_query_timeout: if network_params.network.is_dev_network() {
                Duration::from_secs(1)
            } else {
                Duration::from_secs(60)
            },
            block_processor: BlockProcessorConfig::default(),
            vote_processor: VoteProcessorConfig::new(parallelism),
            tcp: if network_params.network.is_dev_network() {
                TcpConfig::for_dev_network()
            } else {
                Default::default()
            },
            request_aggregator: RequestAggregatorConfig::new(parallelism),
            message_processor: MessageProcessorConfig::new(parallelism),
            priority_scheduler_enabled: true,
            local_block_broadcaster: LocalBlockBroadcasterConfig::new(
                network_params.network.current_network,
            ),
            confirming_set: Default::default(),
            monitor: Default::default(),
        }
    }

    pub fn new_test_instance() -> Self {
        Self::new(None, &DEV_NETWORK_PARAMS, 1)
    }

    pub fn random_representative(&self) -> Account {
        let i = thread_rng().gen_range(0..self.preconfigured_representatives.len());
        return self.preconfigured_representatives[i];
    }
}

pub struct Node {
    pub async_rt: Arc<AsyncRuntime>,
    pub application_path: PathBuf,
    pub relative_time: Instant,
    pub node_id: KeyPair,
    pub config: NodeConfig,
    network_params: NetworkParams,
    pub stats: Arc<Stats>,
    pub workers: Arc<dyn ThreadPool>,
    pub bootstrap_workers: Arc<dyn ThreadPool>,
    wallet_workers: Arc<dyn ThreadPool>,
    election_workers: Arc<dyn ThreadPool>,
    flags: NodeFlags,
    work: Arc<WorkPoolImpl>,
    pub distributed_work: Arc<DistributedWorkFactory>,
    pub store: Arc<LmdbStore>,
    pub unchecked: Arc<UncheckedMap>,
    pub ledger: Arc<Ledger>,
    pub outbound_limiter: Arc<OutboundBandwidthLimiter>,
    pub syn_cookies: Arc<SynCookies>,
    pub network: Arc<Network>,
    pub telemetry: Arc<Telemetry>,
    pub bootstrap_server: Arc<BootstrapServer>,
    pub online_weight_sampler: Arc<OnlineWeightSampler>,
    pub online_reps: Arc<Mutex<OnlineReps>>,
    pub rep_tiers: Arc<RepTiers>,
    pub vote_processor_queue: Arc<VoteProcessorQueue>,
    pub history: Arc<LocalVoteHistory>,
    pub confirming_set: Arc<ConfirmingSet>,
    pub vote_cache: Arc<Mutex<VoteCache>>,
    pub block_processor: Arc<BlockProcessor>,
    pub wallets: Arc<Wallets>,
    pub vote_generators: Arc<VoteGenerators>,
    pub active: Arc<ActiveElections>,
    pub vote_router: Arc<VoteRouter>,
    pub vote_processor: Arc<VoteProcessor>,
    vote_cache_processor: Arc<VoteCacheProcessor>,
    pub websocket: Option<Arc<crate::websocket::WebsocketListener>>,
    pub bootstrap_initiator: Arc<BootstrapInitiator>,
    pub rep_crawler: Arc<RepCrawler>,
    pub tcp_listener: Arc<TcpListener>,
    pub hinted_scheduler: Arc<HintedScheduler>,
    pub manual_scheduler: Arc<ManualScheduler>,
    pub optimistic_scheduler: Arc<OptimisticScheduler>,
    pub priority_scheduler: Arc<PriorityScheduler>,
    pub request_aggregator: Arc<RequestAggregator>,
    pub backlog_population: Arc<BacklogPopulation>,
    pub ascendboot: Arc<BootstrapAscending>,
    pub local_block_broadcaster: Arc<LocalBlockBroadcaster>,
    pub process_live_dispatcher: Arc<ProcessLiveDispatcher>,
    message_processor: Mutex<MessageProcessor>,
    pub network_threads: Arc<Mutex<NetworkThreads>>,
    ledger_pruning: Arc<LedgerPruning>,
    pub peer_connector: Arc<PeerConnector>,
    ongoing_bootstrap: Arc<OngoingBootstrap>,
    peer_cache_updater: TimerThread<PeerCacheUpdater>,
    peer_cache_connector: TimerThread<PeerCacheConnector>,
    pub inbound_message_queue: Arc<InboundMessageQueue>,
    monitor: TimerThread<Monitor>,
    stopped: AtomicBool,
}

impl Node {
    pub fn new(
        async_rt: Arc<AsyncRuntime>,
        application_path: impl Into<PathBuf>,
        config: NodeConfig,
        network_params: NetworkParams,
        flags: NodeFlags,
        work: Arc<WorkPoolImpl>,
        socket_observer: Arc<dyn SocketObserver>,
        election_end: ElectionEndCallback,
        account_balance_changed: AccountBalanceChangedCallback,
        on_vote: Box<
            dyn Fn(&Arc<Vote>, &Option<Arc<ChannelEnum>>, VoteSource, VoteCode) + Send + Sync,
        >,
    ) -> Self {
        let network_label = network_params.network.get_current_network_as_string();
        let global_config = GlobalConfig {
            node_config: config.clone(),
            flags: flags.clone(),
            network_params: network_params.clone(),
        };
        let global_config = &global_config;
        let application_path = application_path.into();
        let node_id = NodeIdKeyFile::default()
            .initialize(&application_path)
            .unwrap();

        let stats = Arc::new(Stats::new(config.stat_config.clone()));

        let store = make_store(
            &application_path,
            true,
            &config.txn_tracking_config.txn_tracking,
            Duration::from_millis(config.block_processor_batch_max_time_ms as u64),
            config.lmdb_config.clone(),
            config.backup_before_upgrade,
        )
        .expect("Could not create LMDB store");

        info!("Version: {}", VERSION_STRING);
        info!("Build information: {}", BUILD_INFO);
        info!("Active network: {}", network_label);
        info!("Database backend: {}", store.vendor());
        info!("Data path: {:?}", application_path);
        info!(
            "Work pool threads: {} ({})",
            work.thread_count(),
            if work.has_opencl() { "OpenCL" } else { "CPU" }
        );
        info!("Work peers: {}", config.work_peers.len());
        info!("Node ID: {}", node_id.public_key().to_node_id());

        let (max_blocks, bootstrap_weights) = if (network_params.network.is_live_network()
            || network_params.network.is_beta_network())
            && !flags.inactive_node
        {
            get_bootstrap_weights(network_params.network.current_network)
        } else {
            (0, HashMap::new())
        };

        let rep_weights = Arc::new(RepWeightCache::with_bootstrap_weights(
            bootstrap_weights,
            max_blocks,
            store.cache.clone(),
        ));

        let mut ledger = Ledger::new(
            store.clone(),
            network_params.ledger.clone(),
            config.representative_vote_weight_minimum,
            rep_weights.clone(),
        )
        .expect("Could not initialize ledger");
        ledger.set_observer(Arc::new(LedgerStats::new(stats.clone())));
        let ledger = Arc::new(ledger);

        log_bootstrap_weights(&ledger.rep_weights);

        let outbound_limiter = Arc::new(OutboundBandwidthLimiter::new(config.borrow().into()));
        let syn_cookies = Arc::new(SynCookies::new(network_params.network.max_peers_per_ip));

        let workers: Arc<dyn ThreadPool> = Arc::new(ThreadPoolImpl::create(
            config.background_threads as usize,
            "Worker".to_string(),
        ));
        let wallet_workers: Arc<dyn ThreadPool> =
            Arc::new(ThreadPoolImpl::create(1, "Wallet work"));
        let election_workers: Arc<dyn ThreadPool> =
            Arc::new(ThreadPoolImpl::create(1, "Election work"));

        let bootstrap_workers: Arc<dyn ThreadPool> = Arc::new(ThreadPoolImpl::create(
            config.bootstrap_serving_threads as usize,
            "Bootstrap work",
        ));

        let inbound_message_queue = Arc::new(InboundMessageQueue::new(
            config.message_processor.max_queue,
            stats.clone(),
        ));
        // empty `config.peering_port` means the user made no port choice at all;
        // otherwise, any value is considered, with `0` having the special meaning of 'let the OS pick a port instead'
        let network = Arc::new(Network::new(NetworkOptions {
            allow_local_peers: config.allow_local_peers,
            tcp_config: config.tcp.clone(),
            publish_filter: Arc::new(NetworkFilter::new(256 * 1024)),
            async_rt: async_rt.clone(),
            network_params: network_params.clone(),
            stats: stats.clone(),
            inbound_queue: inbound_message_queue.clone(),
            port: config.peering_port.unwrap_or(0),
            flags: flags.clone(),
            limiter: outbound_limiter.clone(),
            observer: socket_observer.clone(),
        }));

        let telemetry_config = TelementryConfig {
            enable_ongoing_requests: !flags.disable_ongoing_telemetry_requests,
            enable_ongoing_broadcasts: !flags.disable_providing_telemetry_metrics,
        };

        let unchecked = Arc::new(UncheckedMap::new(
            config.max_unchecked_blocks as usize,
            stats.clone(),
            flags.disable_block_processor_unchecked_deletion,
        ));

        let telemetry = Arc::new(Telemetry::new(
            telemetry_config,
            config.clone(),
            stats.clone(),
            ledger.clone(),
            unchecked.clone(),
            network_params.clone(),
            network.clone(),
            node_id.clone(),
        ));

        let bootstrap_server = Arc::new(BootstrapServer::new(
            config.bootstrap_server.clone(),
            stats.clone(),
            ledger.clone(),
        ));

        let online_weight_sampler = Arc::new(OnlineWeightSampler::new(
            ledger.clone(),
            network_params.node.max_weight_samples as usize,
        ));

        // Time relative to the start of the node. This makes time exlicit and enables us to
        // write time relevant unit tests with ease.
        let relative_time = Instant::now();

        let online_reps = Arc::new(Mutex::new(
            OnlineReps::builder()
                .rep_weights(rep_weights.clone())
                .weight_period(Duration::from_secs(network_params.node.weight_period))
                .online_weight_minimum(config.online_weight_minimum)
                .trended(online_weight_sampler.calculate_trend())
                .finish(),
        ));

        let rep_tiers = Arc::new(RepTiers::new(
            ledger.clone(),
            network_params.clone(),
            online_reps.clone(),
            stats.clone(),
        ));

        let vote_processor_queue = Arc::new(VoteProcessorQueue::new(
            config.vote_processor.clone(),
            stats.clone(),
            rep_tiers.clone(),
        ));

        let history = Arc::new(LocalVoteHistory::new(network_params.voting.max_cache));

        let confirming_set = Arc::new(ConfirmingSet::new(
            config.confirming_set.clone(),
            ledger.clone(),
            stats.clone(),
        ));

        let vote_cache = Arc::new(Mutex::new(VoteCache::new(
            config.vote_cache.clone(),
            stats.clone(),
        )));

        let recently_confirmed = Arc::new(RecentlyConfirmedCache::new(
            config.active_elections.confirmation_cache,
        ));

        let block_processor = Arc::new(BlockProcessor::new(
            global_config.into(),
            ledger.clone(),
            unchecked.clone(),
            stats.clone(),
        ));

        let distributed_work =
            Arc::new(DistributedWorkFactory::new(work.clone(), async_rt.clone()));

        let mut wallets_path = application_path.clone();
        wallets_path.push("wallets.ldb");

        let mut wallets_lmdb_config = config.lmdb_config.clone();
        wallets_lmdb_config.sync = SyncStrategy::Always;
        wallets_lmdb_config.map_size = 1024 * 1024 * 1024;
        let wallets_options = EnvOptions {
            config: wallets_lmdb_config,
            use_no_mem_init: false,
        };
        let wallets_env =
            Arc::new(LmdbEnv::new_with_options(wallets_path, &wallets_options).unwrap());

        let wallets = Arc::new(
            Wallets::new(
                wallets_env,
                ledger.clone(),
                &config,
                network_params.kdf_work,
                network_params.work.clone(),
                distributed_work.clone(),
                network_params.clone(),
                workers.clone(),
                block_processor.clone(),
                online_reps.clone(),
                network.clone(),
                confirming_set.clone(),
            )
            .expect("Could not create wallet"),
        );
        wallets.initialize2();

        let inbound_impl: Arc<
            RwLock<Box<dyn Fn(DeserializedMessage, Arc<ChannelEnum>) + Send + Sync>>,
        > = Arc::new(RwLock::new(Box::new(|_msg, _channel| {
            panic!("inbound callback not set");
        })));
        let inbound_impl_clone = inbound_impl.clone();
        let inbound: InboundCallback =
            Arc::new(move |msg: DeserializedMessage, channel: Arc<ChannelEnum>| {
                let cb = inbound_impl_clone.read().unwrap();
                (*cb)(msg, channel);
            });

        let loopback_channel = create_loopback_channel(
            node_id.public_key(),
            &network,
            stats.clone(),
            &network_params,
            inbound,
            &async_rt,
        );

        let vote_broadcaster = Arc::new(VoteBroadcaster::new(
            online_reps.clone(),
            network.clone(),
            vote_processor_queue.clone(),
            loopback_channel,
        ));

        let vote_generators = Arc::new(VoteGenerators::new(
            ledger.clone(),
            wallets.clone(),
            history.clone(),
            stats.clone(),
            &config,
            &network_params,
            vote_broadcaster,
        ));

        let vote_applier = Arc::new(VoteApplier::new(
            ledger.clone(),
            network_params.clone(),
            online_reps.clone(),
            stats.clone(),
            vote_generators.clone(),
            block_processor.clone(),
            config.clone(),
            history.clone(),
            wallets.clone(),
            recently_confirmed.clone(),
            confirming_set.clone(),
            election_workers.clone(),
        ));

        let vote_router = Arc::new(VoteRouter::new(
            recently_confirmed.clone(),
            vote_applier.clone(),
        ));

        let vote_processor = Arc::new(VoteProcessor::new(
            vote_processor_queue.clone(),
            vote_router.clone(),
            stats.clone(),
            on_vote,
        ));

        let vote_cache_processor = Arc::new(VoteCacheProcessor::new(
            stats.clone(),
            vote_cache.clone(),
            vote_router.clone(),
            config.vote_processor.clone(),
        ));

        let active_elections = Arc::new(ActiveElections::new(
            network_params.clone(),
            wallets.clone(),
            config.clone(),
            ledger.clone(),
            confirming_set.clone(),
            block_processor.clone(),
            vote_generators.clone(),
            network.clone(),
            vote_cache.clone(),
            stats.clone(),
            election_end,
            account_balance_changed,
            online_reps.clone(),
            flags.clone(),
            recently_confirmed,
            vote_applier,
            vote_router.clone(),
            vote_cache_processor.clone(),
            relative_time,
        ));

        active_elections.initialize();
        let websocket = create_websocket_server(
            config.websocket_config.clone(),
            wallets.clone(),
            async_rt.clone(),
            &active_elections,
            &telemetry,
            &vote_processor,
        );

        let bootstrap_initiator = Arc::new(BootstrapInitiator::new(
            global_config.into(),
            flags.clone(),
            network.clone(),
            async_rt.clone(),
            bootstrap_workers.clone(),
            network_params.clone(),
            socket_observer.clone(),
            stats.clone(),
            outbound_limiter.clone(),
            block_processor.clone(),
            websocket.clone(),
            ledger.clone(),
        ));
        bootstrap_initiator.initialize();
        bootstrap_initiator.start();

        let response_server_factory = Arc::new(ResponseServerFactory {
            runtime: async_rt.clone(),
            stats: stats.clone(),
            node_id: node_id.clone(),
            ledger: ledger.clone(),
            workers: workers.clone(),
            block_processor: block_processor.clone(),
            bootstrap_initiator: bootstrap_initiator.clone(),
            network: network.clone(),
            inbound_queue: inbound_message_queue.clone(),
            node_flags: flags.clone(),
            network_params: network_params.clone(),
            syn_cookies: syn_cookies.clone(),
        });

        let peer_connector = Arc::new(PeerConnector::new(
            config.tcp.clone(),
            config.clone(),
            network.clone(),
            stats.clone(),
            async_rt.clone(),
            socket_observer.clone(),
            workers.clone(),
            network_params.clone(),
            response_server_factory.clone(),
        ));

        let rep_crawler = Arc::new(RepCrawler::new(
            online_reps.clone(),
            stats.clone(),
            config.rep_crawler_query_timeout,
            config.clone(),
            network_params.clone(),
            network.clone(),
            async_rt.clone(),
            ledger.clone(),
            active_elections.clone(),
            peer_connector.clone(),
            relative_time,
        ));

        // BEWARE: `bootstrap` takes `network.port` instead of `config.peering_port` because when the user doesn't specify
        //         a peering port and wants the OS to pick one, the picking happens when `network` gets initialized
        //         (if UDP is active, otherwise it happens when `bootstrap` gets initialized), so then for TCP traffic
        //         we want to tell `bootstrap` to use the already picked port instead of itself picking a different one.
        //         Thus, be very careful if you change the order: if `bootstrap` gets constructed before `network`,
        //         the latter would inherit the port from the former (if TCP is active, otherwise `network` picks first)
        //
        let tcp_listener = Arc::new(TcpListener::new(
            network.port(),
            config.clone(),
            network.clone(),
            network_params.clone(),
            async_rt.clone(),
            socket_observer,
            stats.clone(),
            workers.clone(),
            response_server_factory.clone(),
        ));

        let hinted_scheduler = Arc::new(HintedScheduler::new(
            config.hinted_scheduler.clone(),
            active_elections.clone(),
            ledger.clone(),
            stats.clone(),
            vote_cache.clone(),
            confirming_set.clone(),
            online_reps.clone(),
        ));

        let manual_scheduler = Arc::new(ManualScheduler::new(
            stats.clone(),
            active_elections.clone(),
        ));

        let optimistic_scheduler = Arc::new(OptimisticScheduler::new(
            config.optimistic_scheduler.clone(),
            stats.clone(),
            active_elections.clone(),
            network_params.network.clone(),
            ledger.clone(),
            confirming_set.clone(),
        ));

        let priority_scheduler = Arc::new(PriorityScheduler::new(
            config.priority_bucket.clone(),
            ledger.clone(),
            stats.clone(),
            active_elections.clone(),
        ));

        let priority_clone = Arc::downgrade(&priority_scheduler);
        active_elections.set_activate_successors_callback(Box::new(move |tx, block| {
            if let Some(priority) = priority_clone.upgrade() {
                priority.activate_successors(tx, block);
            }
        }));

        let request_aggregator = Arc::new(RequestAggregator::new(
            config.request_aggregator.clone(),
            stats.clone(),
            vote_generators.clone(),
            ledger.clone(),
        ));

        let backlog_population = Arc::new(BacklogPopulation::new(
            global_config.into(),
            ledger.clone(),
            stats.clone(),
            optimistic_scheduler.clone(),
            priority_scheduler.clone(),
        ));

        let ascendboot = Arc::new(BootstrapAscending::new(
            block_processor.clone(),
            ledger.clone(),
            stats.clone(),
            network.clone(),
            global_config.into(),
        ));

        let local_block_broadcaster = Arc::new(LocalBlockBroadcaster::new(
            config.local_block_broadcaster.clone(),
            block_processor.clone(),
            stats.clone(),
            network.clone(),
            online_reps.clone(),
            ledger.clone(),
            confirming_set.clone(),
            !flags.disable_block_processor_republishing,
        ));
        local_block_broadcaster.initialize();

        let process_live_dispatcher = Arc::new(ProcessLiveDispatcher::new(
            ledger.clone(),
            priority_scheduler.clone(),
            websocket.clone(),
        ));

        let realtime_message_handler = Arc::new(RealtimeMessageHandler::new(
            stats.clone(),
            network.clone(),
            peer_connector.clone(),
            block_processor.clone(),
            config.clone(),
            flags.clone(),
            wallets.clone(),
            request_aggregator.clone(),
            vote_processor_queue.clone(),
            telemetry.clone(),
            bootstrap_server.clone(),
            ascendboot.clone(),
        ));

        let realtime_message_handler_weak = Arc::downgrade(&realtime_message_handler);
        *inbound_impl.write().unwrap() =
            Box::new(move |msg: DeserializedMessage, channel: Arc<ChannelEnum>| {
                if let Some(handler) = realtime_message_handler_weak.upgrade() {
                    handler.process(msg.message, &channel);
                }
            });

        let keepalive_factory = Arc::new(KeepaliveFactory {
            network: network.clone(),
            config: config.clone(),
        });
        let network_threads = Arc::new(Mutex::new(NetworkThreads::new(
            network.clone(),
            peer_connector.clone(),
            flags.clone(),
            network_params.clone(),
            stats.clone(),
            syn_cookies.clone(),
            keepalive_factory.clone(),
            online_reps.clone(),
        )));

        let message_processor = Mutex::new(MessageProcessor::new(
            flags.clone(),
            config.clone(),
            inbound_message_queue.clone(),
            realtime_message_handler.clone(),
        ));

        let ongoing_bootstrap = Arc::new(OngoingBootstrap::new(
            network_params.clone(),
            bootstrap_initiator.clone(),
            network.clone(),
            flags.clone(),
            ledger.clone(),
            stats.clone(),
            workers.clone(),
        ));

        debug!("Constructing node...");

        let manual_weak = Arc::downgrade(&manual_scheduler);
        wallets.set_start_election_callback(Box::new(move |block| {
            if let Some(manual) = manual_weak.upgrade() {
                manual.push(block, None);
            }
        }));

        let rep_crawler_w = Arc::downgrade(&rep_crawler);
        if !flags.disable_rep_crawler {
            network.on_new_channel(Arc::new(move |channel| {
                if let Some(crawler) = rep_crawler_w.upgrade() {
                    crawler.query_channel(channel);
                }
            }));
        }

        let block_processor_w = Arc::downgrade(&block_processor);
        let history_w = Arc::downgrade(&history);
        let active_w = Arc::downgrade(&active_elections);
        block_processor.set_blocks_rolled_back_callback(Box::new(
            move |rolled_back, initial_block| {
                // Deleting from votes cache, stop active transaction
                let Some(block_processor) = block_processor_w.upgrade() else {
                    return;
                };
                let Some(history) = history_w.upgrade() else {
                    return;
                };
                let Some(active) = active_w.upgrade() else {
                    return;
                };
                for i in rolled_back {
                    block_processor.notify_block_rolled_back(&i);

                    history.erase(&i.root());
                    // Stop all rolled back active transactions except initial
                    if i.hash() != initial_block.hash() {
                        active.erase(&i.qualified_root());
                    }
                }
            },
        ));

        process_live_dispatcher.connect(&block_processor);

        let block_processor_w = Arc::downgrade(&block_processor);
        unchecked.set_satisfied_observer(Box::new(move |info| {
            if let Some(processor) = block_processor_w.upgrade() {
                processor.add(
                    info.block.as_ref().unwrap().clone(),
                    BlockSource::Unchecked,
                    None,
                );
            }
        }));

        let ledger_w = Arc::downgrade(&ledger);
        let vote_cache_w = Arc::downgrade(&vote_cache);
        let wallets_w = Arc::downgrade(&wallets);
        let channels_w = Arc::downgrade(&network);
        vote_router.add_vote_processed_observer(Box::new(move |vote, source, results| {
            let Some(ledger) = ledger_w.upgrade() else {
                return;
            };
            let Some(vote_cache) = vote_cache_w.upgrade() else {
                return;
            };
            let Some(wallets) = wallets_w.upgrade() else {
                return;
            };
            let Some(channels) = channels_w.upgrade() else {
                return;
            };
            let rep_weight = ledger.weight(&vote.voting_account);

            if source != VoteSource::Cache {
                vote_cache.lock().unwrap().insert(vote, rep_weight, results);
            }

            // Republish vote if it is new and the node does not host a principal representative (or close to)
            let processed = results.iter().any(|(_, code)| *code == VoteCode::Vote);
            if processed {
                if wallets.should_republish_vote(vote.voting_account) {
                    let ack = Message::ConfirmAck(ConfirmAck::new_with_rebroadcasted_vote(
                        vote.as_ref().clone(),
                    ));
                    channels.flood_message(&ack, 0.5);
                }
            }
        }));

        let priority_w = Arc::downgrade(&priority_scheduler);
        let hinted_w = Arc::downgrade(&hinted_scheduler);
        let optimistic_w = Arc::downgrade(&optimistic_scheduler);
        // Notify election schedulers when AEC frees election slot
        *active_elections.vacancy_update.lock().unwrap() = Box::new(move || {
            let Some(priority) = priority_w.upgrade() else {
                return;
            };
            let Some(hinted) = hinted_w.upgrade() else {
                return;
            };
            let Some(optimistic) = optimistic_w.upgrade() else {
                return;
            };

            priority.notify();
            hinted.notify();
            optimistic.notify();
        });

        let keepalive_factory_w = Arc::downgrade(&keepalive_factory);
        network.on_new_channel(Arc::new(move |channel| {
            let Some(factory) = keepalive_factory_w.upgrade() else {
                return;
            };
            let keepalive = factory.create_keepalive_self();
            let msg = Message::Keepalive(keepalive);
            channel.send(&msg, None, BufferDropPolicy::Limiter, TrafficType::Generic);
        }));

        let rep_crawler_w = Arc::downgrade(&rep_crawler);
        let reps_w = Arc::downgrade(&online_reps);
        vote_processor.add_vote_processed_callback(Box::new(move |vote, channel, source, code| {
            debug_assert!(code != VoteCode::Invalid);
            let Some(rep_crawler) = rep_crawler_w.upgrade() else {
                return;
            };
            let Some(reps) = reps_w.upgrade() else {
                return;
            };
            let Some(channel) = &channel else {
                return; // Channel expired when waiting for vote to be processed
            };
            // Ignore republished votes
            if source != VoteSource::Live {
                return;
            }

            let active_in_rep_crawler = rep_crawler.process(vote.clone(), channel.clone());
            if active_in_rep_crawler {
                // Representative is defined as online if replying to live votes or rep_crawler queries
                reps.lock()
                    .unwrap()
                    .vote_observed(vote.voting_account, relative_time.elapsed());
            }
        }));

        if !distributed_work.work_generation_enabled() {
            info!("Work generation is disabled");
        }

        info!(
            "Outbound bandwidth limit: {} bytes/s, burst ratio: {}",
            config.bandwidth_limit, config.bandwidth_limit_burst_ratio
        );

        if !ledger
            .any()
            .block_exists_or_pruned(&ledger.read_txn(), &network_params.ledger.genesis.hash())
        {
            error!("Genesis block not found. This commonly indicates a configuration issue, check that the --network or --data_path command line arguments are correct, and also the ledger backend node config option. If using a read-only CLI command a ledger must already exist, start the node with --daemon first.");

            if network_params.network.is_beta_network() {
                error!("Beta network may have reset, try clearing database files");
            }

            panic!("Genesis block not found!");
        }

        if config.enable_voting {
            info!(
                "Voting is enabled, more system resources will be used, local representatives: {}",
                wallets.voting_reps_count()
            );
            if wallets.voting_reps_count() > 1 {
                warn!("Voting with more than one representative can limit performance");
            }
        }

        {
            let tx = ledger.read_txn();
            if flags.enable_pruning || ledger.store.pruned.count(&tx) > 0 {
                ledger.enable_pruning();
            }
        }

        if ledger.pruning_enabled() {
            if config.enable_voting && !flags.inactive_node {
                let msg = "Incompatibility detected between config node.enable_voting and existing pruned blocks";
                error!(msg);
                panic!("{}", msg);
            } else if !flags.enable_pruning && !flags.inactive_node {
                let msg =
                    "To start node with existing pruned blocks use launch flag --enable_pruning";
                error!(msg);
                panic!("{}", msg);
            }
        }

        let workers_w = Arc::downgrade(&wallet_workers);
        let wallets_w = Arc::downgrade(&wallets);
        confirming_set.add_cemented_observer(Box::new(move |block| {
            let Some(workers) = workers_w.upgrade() else {
                return;
            };
            let Some(wallets) = wallets_w.upgrade() else {
                return;
            };

            // TODO: Is it neccessary to call this for all blocks?
            if block.is_send() {
                let block = block.clone();
                workers.push_task(Box::new(move || {
                    wallets.receive_confirmed(block.hash(), block.destination().unwrap())
                }));
            }
        }));

        if !config.callback_address.is_empty() {
            let async_rt = async_rt.clone();
            let stats = stats.clone();
            let url: Url = format!(
                "http://{}:{}{}",
                config.callback_address, config.callback_port, config.callback_target
            )
            .parse()
            .unwrap();
            active_elections.add_election_end_callback(Box::new(
                move |status, _weights, account, amount, is_state_send, is_state_epoch| {
                    let block = status.winner.as_ref().unwrap().clone();
                    if status.election_status_type == ElectionStatusType::ActiveConfirmedQuorum
                        || status.election_status_type
                            == ElectionStatusType::ActiveConfirmationHeight
                    {
                        let url = url.clone();
                        let stats = stats.clone();
                        async_rt.tokio.spawn(async move {
                            let mut block_json = SerdePropertyTree::new();
                            block.serialize_json(&mut block_json).unwrap();

                            let message = RpcCallbackMessage {
                                account: account.encode_account(),
                                hash: block.hash().encode_hex(),
                                block: block_json.value,
                                amount: amount.to_string_dec(),
                                sub_type: if is_state_send {
                                    Some("send")
                                } else if block.block_type() == BlockType::State {
                                    if block.is_change() {
                                        Some("change")
                                    } else if is_state_epoch {
                                        Some("epoch")
                                    } else {
                                        Some("receive")
                                    }
                                } else {
                                    None
                                },
                                is_send: if is_state_send {
                                    Some(as_nano_json(true))
                                } else {
                                    None
                                },
                            };

                            let http_client = HttpClient::new();
                            match http_client.post_json(url.clone(), &message).await {
                                Ok(response) => {
                                    if response.status().is_success() {
                                        stats.inc_dir(
                                            StatType::HttpCallback,
                                            DetailType::Initiate,
                                            Direction::Out,
                                        );
                                    } else {
                                        error!(
                                            "Callback to {} failed [status: {:?}]",
                                            url,
                                            response.status()
                                        );
                                        stats.inc_dir(
                                            StatType::Error,
                                            DetailType::HttpCallback,
                                            Direction::Out,
                                        );
                                    }
                                }
                                Err(e) => {
                                    error!("Unable to send callback: {} ({})", url, e);
                                    stats.inc_dir(
                                        StatType::Error,
                                        DetailType::HttpCallback,
                                        Direction::Out,
                                    );
                                }
                            }
                        });
                    }
                },
            ))
        }

        let time_factory = SystemTimeFactory::default();

        let peer_cache_updater = PeerCacheUpdater::new(
            network.clone(),
            ledger.clone(),
            time_factory,
            stats.clone(),
            if network_params.network.is_dev_network() {
                Duration::from_secs(10)
            } else {
                Duration::from_secs(60 * 60)
            },
        );

        let peer_cache_connector = PeerCacheConnector::new(
            ledger.clone(),
            peer_connector.clone(),
            stats.clone(),
            network_params.network.merge_period,
        );

        let ledger_pruning = Arc::new(LedgerPruning::new(
            config.clone(),
            flags.clone(),
            ledger.clone(),
            workers.clone(),
        ));

        let monitor = TimerThread::new(
            "Monitor",
            Monitor::new(
                ledger.clone(),
                network.clone(),
                online_reps.clone(),
                active_elections.clone(),
            ),
        );

        Self {
            relative_time,
            peer_cache_updater: TimerThread::new("Peer history", peer_cache_updater),
            peer_cache_connector: TimerThread::new_run_immedately(
                "Net reachout",
                peer_cache_connector,
            ),
            ongoing_bootstrap,
            peer_connector,
            node_id,
            workers,
            bootstrap_workers,
            wallet_workers,
            election_workers,
            distributed_work,
            unchecked,
            telemetry,
            outbound_limiter,
            syn_cookies,
            network,
            ledger,
            store,
            stats,
            application_path,
            network_params,
            config,
            flags,
            work,
            async_rt,
            bootstrap_server,
            online_weight_sampler,
            online_reps,
            rep_tiers,
            vote_router,
            vote_processor_queue,
            history,
            confirming_set,
            vote_cache,
            block_processor,
            wallets,
            vote_generators,
            active: active_elections,
            vote_processor,
            vote_cache_processor,
            websocket,
            bootstrap_initiator,
            rep_crawler,
            tcp_listener,
            hinted_scheduler,
            manual_scheduler,
            optimistic_scheduler,
            priority_scheduler,
            request_aggregator,
            backlog_population,
            ascendboot,
            local_block_broadcaster,
            process_live_dispatcher,
            ledger_pruning,
            network_threads,
            message_processor,
            inbound_message_queue,
            monitor,
            stopped: AtomicBool::new(false),
        }
    }

    pub fn collect_container_info(&self, name: impl Into<String>) -> ContainerInfoComponent {
        ContainerInfoComponent::Composite(
            name.into(),
            vec![
                self.work.collect_container_info("work"),
                self.ledger.collect_container_info("ledger"),
                self.active.collect_container_info("active"),
                self.bootstrap_initiator
                    .collect_container_info("bootstrap_initiator"),
                ContainerInfoComponent::Composite(
                    "network".to_string(),
                    vec![
                        self.network.collect_container_info("tcp_channels"),
                        self.syn_cookies.collect_container_info("syn_cookies"),
                    ],
                ),
                self.telemetry.collect_container_info("telemetry"),
                self.wallets.collect_container_info("wallets"),
                self.vote_processor_queue
                    .collect_container_info("vote_processor"),
                self.vote_cache_processor
                    .collect_container_info("vote_cache_processor"),
                self.rep_crawler.collect_container_info("rep_crawler"),
                self.block_processor
                    .collect_container_info("block_processor"),
                self.online_reps
                    .lock()
                    .unwrap()
                    .collect_container_info("online_reps"),
                self.history.collect_container_info("history"),
                self.confirming_set.collect_container_info("confirming_set"),
                self.request_aggregator
                    .collect_container_info("request_aggregator"),
                ContainerInfoComponent::Composite(
                    "election_scheduler".to_string(),
                    vec![
                        self.hinted_scheduler.collect_container_info("hinted"),
                        self.manual_scheduler.collect_container_info("manual"),
                        self.optimistic_scheduler
                            .collect_container_info("optimistic"),
                        self.priority_scheduler.collect_container_info("priority"),
                    ],
                ),
                self.vote_cache
                    .lock()
                    .unwrap()
                    .collect_container_info("vote_cache"),
                self.vote_router.collect_container_info("vote_router"),
                self.vote_generators
                    .collect_container_info("vote_generators"),
                self.ascendboot
                    .collect_container_info("bootstrap_ascending"),
                self.unchecked.collect_container_info("unchecked"),
                self.local_block_broadcaster
                    .collect_container_info("local_block_broadcaster"),
                self.rep_tiers.collect_container_info("rep_tiers"),
                self.inbound_message_queue
                    .collect_container_info("message_processor"),
            ],
        )
    }

    fn long_inactivity_cleanup(&self) {
        let mut perform_cleanup = false;
        let mut tx = self.ledger.rw_txn();
        if self.ledger.store.online_weight.count(&tx) > 0 {
            let (&sample_time, _) = self
                .ledger
                .store
                .online_weight
                .rbegin(&tx)
                .current()
                .unwrap();
            let one_week_ago = SystemTime::now() - Duration::from_secs(60 * 60 * 24 * 7);
            perform_cleanup = sample_time < system_time_as_nanoseconds(one_week_ago);
        }
        if perform_cleanup {
            self.ledger.store.online_weight.clear(&mut tx);
            self.ledger.store.peer.clear(&mut tx);
            info!("records of peers and online weight after a long period of inactivity");
        }
    }

    pub fn is_stopped(&self) -> bool {
        self.stopped.load(Ordering::SeqCst)
    }

    pub fn ledger_pruning(&self, batch_size: u64, bootstrap_weight_reached: bool) {
        self.ledger_pruning
            .ledger_pruning(batch_size, bootstrap_weight_reached)
    }

    pub fn process_local(&self, block: BlockEnum) -> Option<BlockStatus> {
        self.block_processor
            .add_blocking(Arc::new(block), BlockSource::Local)
    }

    pub fn process_multi(&self, blocks: &[BlockEnum]) {
        let mut tx = self.ledger.rw_txn();
        for block in blocks {
            self.ledger.process(&mut tx, &mut block.clone()).unwrap();
        }
    }

    pub fn process_active(&self, block: BlockEnum) {
        self.block_processor.process_active(Arc::new(block));
    }

    pub fn process_local_multi(&self, blocks: &[BlockEnum]) {
        for block in blocks {
            let status = self.process_local(block.clone()).unwrap();
            if !matches!(status, BlockStatus::Progress | BlockStatus::Old) {
                panic!("could not process block!");
            }
        }
    }

    pub fn block(&self, hash: &BlockHash) -> Option<BlockEnum> {
        let tx = self.ledger.read_txn();
        self.ledger.any().get_block(&tx, hash)
    }

    pub fn get_node_id(&self) -> PublicKey {
        self.node_id.public_key()
    }

    pub fn work_generate_dev(&self, root: Root) -> u64 {
        self.work.generate_dev2(root).unwrap()
    }

    pub fn block_exists(&self, hash: &BlockHash) -> bool {
        let tx = self.ledger.read_txn();
        self.ledger.any().block_exists(&tx, hash)
    }

    pub fn blocks_exist(&self, hashes: &[BlockEnum]) -> bool {
        self.block_hashes_exist(hashes.iter().map(|b| b.hash()))
    }

    pub fn block_hashes_exist(&self, hashes: impl IntoIterator<Item = BlockHash>) -> bool {
        let tx = self.ledger.read_txn();
        hashes
            .into_iter()
            .all(|h| self.ledger.any().block_exists(&tx, &h))
    }

    pub fn balance(&self, account: &Account) -> Amount {
        let tx = self.ledger.read_txn();
        self.ledger
            .any()
            .account_balance(&tx, account)
            .unwrap_or_default()
    }

    pub fn confirm_multi(&self, blocks: &[BlockEnum]) {
        for block in blocks {
            self.confirm(block.hash());
        }
    }

    pub fn confirm(&self, hash: BlockHash) {
        let mut tx = self.ledger.rw_txn();
        self.ledger.confirm(&mut tx, hash);
    }

    pub fn blocks_confirmed(&self, blocks: &[BlockEnum]) -> bool {
        let tx = self.ledger.read_txn();
        blocks
            .iter()
            .all(|b| self.ledger.confirmed().block_exists(&tx, &b.hash()))
    }
}

pub trait NodeExt {
    fn start(&self);
    fn stop(&self);
    fn ongoing_online_weight_calculation_queue(&self);
    fn ongoing_online_weight_calculation(&self);
    fn backup_wallet(&self);
    fn search_receivable_all(&self);
    fn bootstrap_wallet(&self);
}

impl NodeExt for Arc<Node> {
    fn start(&self) {
        self.long_inactivity_cleanup();
        self.network_threads.lock().unwrap().start();
        self.message_processor.lock().unwrap().start();

        if !self.flags.disable_legacy_bootstrap && !self.flags.disable_ongoing_bootstrap {
            self.ongoing_bootstrap.ongoing_bootstrap();
        }

        if self.flags.enable_pruning {
            self.ledger_pruning.start();
        }

        if !self.flags.disable_rep_crawler {
            self.rep_crawler.start();
        }
        self.ongoing_online_weight_calculation_queue();

        if self.config.tcp_incoming_connections_max > 0
            && !(self.flags.disable_bootstrap_listener && self.flags.disable_tcp_realtime)
        {
            self.tcp_listener.start();
        } else {
            warn!("Peering is disabled");
        }

        if !self.flags.disable_backup {
            self.backup_wallet();
        }

        if !self.flags.disable_search_pending {
            self.search_receivable_all();
        }

        if !self.flags.disable_wallet_bootstrap {
            // Delay to start wallet lazy bootstrap
            let node_w = Arc::downgrade(self);
            self.workers.add_delayed_task(
                Duration::from_secs(60),
                Box::new(move || {
                    if let Some(node) = node_w.upgrade() {
                        node.bootstrap_wallet();
                    }
                }),
            );
        }

        self.unchecked.start();
        self.wallets.start();
        self.rep_tiers.start();
        self.vote_processor.start();
        self.vote_cache_processor.start();
        self.block_processor.start();
        self.active.start();
        self.vote_generators.start();
        self.request_aggregator.start();
        self.confirming_set.start();
        self.hinted_scheduler.start();
        self.manual_scheduler.start();
        self.optimistic_scheduler.start();
        if self.config.priority_scheduler_enabled {
            self.priority_scheduler.start();
        }
        self.backlog_population.start();
        self.bootstrap_server.start();
        if !self.flags.disable_ascending_bootstrap {
            self.ascendboot.start();
        }
        if let Some(ws_listener) = &self.websocket {
            ws_listener.start();
        }
        self.telemetry.start();
        self.stats.start();
        self.local_block_broadcaster.start();

        let peer_cache_update_interval = if self.network_params.network.is_dev_network() {
            Duration::from_secs(1)
        } else {
            Duration::from_secs(15)
        };
        self.peer_cache_updater.start(peer_cache_update_interval);

        if !self.network_params.network.merge_period.is_zero() {
            self.peer_cache_connector
                .start(self.network_params.network.merge_period);
        }
        self.vote_router.start();

        if self.config.monitor.enabled {
            self.monitor.start(self.config.monitor.interval);
        }
    }

    fn stop(&self) {
        // Ensure stop can only be called once
        if self.stopped.swap(true, Ordering::SeqCst) {
            return;
        }
        info!("Node stopping...");

        self.tcp_listener.stop();
        self.bootstrap_workers.stop();
        self.wallet_workers.stop();
        self.election_workers.stop();
        self.vote_router.stop();
        self.peer_connector.stop();
        self.ledger_pruning.stop();
        self.peer_cache_connector.stop();
        self.peer_cache_updater.stop();
        // Cancels ongoing work generation tasks, which may be blocking other threads
        // No tasks may wait for work generation in I/O threads, or termination signal capturing will be unable to call node::stop()
        self.distributed_work.stop();
        self.backlog_population.stop();
        if !self.flags.disable_ascending_bootstrap {
            self.ascendboot.stop();
        }
        self.rep_crawler.stop();
        self.unchecked.stop();
        self.block_processor.stop();
        self.request_aggregator.stop();
        self.vote_cache_processor.stop();
        self.vote_processor.stop();
        self.rep_tiers.stop();
        self.hinted_scheduler.stop();
        self.manual_scheduler.stop();
        self.optimistic_scheduler.stop();
        self.priority_scheduler.stop();
        self.active.stop();
        self.vote_generators.stop();
        self.confirming_set.stop();
        self.telemetry.stop();
        if let Some(ws_listener) = &self.websocket {
            ws_listener.stop();
        }
        self.bootstrap_server.stop();
        self.bootstrap_initiator.stop();
        self.wallets.stop();
        self.stats.stop();
        self.workers.stop();
        self.local_block_broadcaster.stop();
        self.message_processor.lock().unwrap().stop();
        self.network_threads.lock().unwrap().stop(); // Stop network last to avoid killing in-use sockets
        self.monitor.stop();

        // work pool is not stopped on purpose due to testing setup
    }

    fn ongoing_online_weight_calculation_queue(&self) {
        let node_w = Arc::downgrade(self);
        self.workers.add_delayed_task(
            Duration::from_secs(self.network_params.node.weight_period),
            Box::new(move || {
                if let Some(node) = node_w.upgrade() {
                    node.ongoing_online_weight_calculation();
                }
            }),
        )
    }

    fn ongoing_online_weight_calculation(&self) {
        let online = self.online_reps.lock().unwrap().online_weight();
        self.online_weight_sampler.sample(online);
        let trend = self.online_weight_sampler.calculate_trend();
        self.online_reps.lock().unwrap().set_trended(trend);
    }

    fn backup_wallet(&self) {
        let mut backup_path = self.application_path.clone();
        backup_path.push("backup");
        if let Err(e) = self.wallets.backup(&backup_path) {
            error!(error = ?e, "Could not create backup of wallets");
        }

        let node_w = Arc::downgrade(self);
        self.workers.add_delayed_task(
            Duration::from_secs(self.network_params.node.backup_interval_m as u64 * 60),
            Box::new(move || {
                if let Some(node) = node_w.upgrade() {
                    node.backup_wallet();
                }
            }),
        )
    }

    fn search_receivable_all(&self) {
        // Reload wallets from disk
        self.wallets.reload();
        // Search pending
        self.wallets.search_receivable_all();
        let node_w = Arc::downgrade(self);
        self.workers.add_delayed_task(
            Duration::from_secs(self.network_params.node.search_pending_interval_s as u64),
            Box::new(move || {
                if let Some(node) = node_w.upgrade() {
                    node.search_receivable_all();
                }
            }),
        )
    }

    fn bootstrap_wallet(&self) {
        let accounts: VecDeque<_> = self.wallets.get_accounts(128).drain(..).collect();
        if !accounts.is_empty() {
            self.bootstrap_initiator.bootstrap_wallet(accounts)
        }
    }
}

fn make_store(
    path: &Path,
    add_db_postfix: bool,
    txn_tracking_config: &TxnTrackingConfig,
    block_processor_batch_max_time: Duration,
    lmdb_config: LmdbConfig,
    backup_before_upgrade: bool,
) -> anyhow::Result<Arc<LmdbStore>> {
    let mut path = PathBuf::from(path);
    if add_db_postfix {
        path.push("data.ldb");
    }

    let txn_tracker: Arc<dyn TransactionTracker> = if txn_tracking_config.enable {
        Arc::new(LongRunningTransactionLogger::new(
            txn_tracking_config.clone(),
            block_processor_batch_max_time,
        ))
    } else {
        Arc::new(NullTransactionTracker::new())
    };

    let options = EnvOptions {
        config: lmdb_config,
        use_no_mem_init: true,
    };

    let store = LmdbStore::open(&path)
        .options(&options)
        .backup_before_upgrade(backup_before_upgrade)
        .txn_tracker(txn_tracker)
        .build()?;
    Ok(Arc::new(store))
}

#[derive(Serialize)]
struct RpcCallbackMessage {
    account: String,
    hash: String,
    block: serde_json::Value,
    amount: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    sub_type: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_send: Option<&'static str>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{transport::NullSocketObserver, utils::TimerStartEvent};
    use rsnano_core::Networks;
    use std::ops::Deref;
    use uuid::Uuid;

    #[test]
    fn start_peer_cache_updater() {
        let node = TestNode::new();
        let start_tracker = node.peer_cache_updater.track_start();

        node.start();

        assert_eq!(
            start_tracker.output(),
            vec![TimerStartEvent {
                thread_name: "Peer history".to_string(),
                interval: Duration::from_secs(1),
                run_immediately: false
            }]
        );
    }

    #[test]
    fn start_peer_cache_connector() {
        let node = TestNode::new();
        let start_tracker = node.peer_cache_connector.track_start();

        node.start();

        assert_eq!(
            start_tracker.output(),
            vec![TimerStartEvent {
                thread_name: "Net reachout".to_string(),
                interval: node.network_params.network.merge_period,
                run_immediately: true
            }]
        );
    }

    #[test]
    fn stop_node() {
        let node = TestNode::new();
        node.start();

        node.stop();

        assert_eq!(
            node.peer_cache_updater.is_running(),
            false,
            "peer_cache_updater running"
        );
        assert_eq!(
            node.peer_cache_connector.is_running(),
            false,
            "peer_cache_connector running"
        );
    }

    struct TestNode {
        app_path: PathBuf,
        node: Arc<Node>,
    }

    impl TestNode {
        pub fn new() -> Self {
            let async_rt = Arc::new(AsyncRuntime::default());
            let mut app_path = std::env::temp_dir();
            app_path.push(format!("rsnano-test-{}", Uuid::new_v4().simple()));
            let config = NodeConfig::new_test_instance();
            let network_params = NetworkParams::new(Networks::NanoDevNetwork);
            let flags = NodeFlags::default();
            let work = Arc::new(WorkPoolImpl::new(
                network_params.work.clone(),
                1,
                Duration::ZERO,
            ));

            let node = Arc::new(Node::new(
                async_rt,
                &app_path,
                config,
                network_params,
                flags,
                work,
                Arc::new(NullSocketObserver::new()),
                Box::new(|_, _, _, _, _, _| {}),
                Box::new(|_, _| {}),
                Box::new(|_, _, _, _| {}),
            ));

            Self { node, app_path }
        }
    }

    impl Drop for TestNode {
        fn drop(&mut self) {
            self.node.stop();
            std::fs::remove_dir_all(&self.app_path).unwrap();
        }
    }

    impl Deref for TestNode {
        type Target = Arc<Node>;

        fn deref(&self) -> &Self::Target {
            &self.node
        }
    }
}
