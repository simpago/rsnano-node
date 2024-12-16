use crate::{
    block_processing::{
        BacklogPopulation, BlockProcessor, BlockProcessorCleanup, BlockSource,
        LocalBlockBroadcaster, LocalBlockBroadcasterExt, UncheckedMap,
    },
    bootstrap::{BootstrapExt, BootstrapServer, BootstrapServerCleanup, BootstrapService},
    cementation::ConfirmingSet,
    config::{GlobalConfig, NodeConfig, NodeFlags},
    consensus::{
        election_schedulers::ElectionSchedulers, get_bootstrap_weights, log_bootstrap_weights,
        ActiveElections, ActiveElectionsExt, ElectionStatusType, LocalVoteHistory,
        ProcessLiveDispatcher, ProcessLiveDispatcherExt, RecentlyConfirmedCache, RepTiers,
        RequestAggregator, RequestAggregatorCleanup, VoteApplier, VoteBroadcaster, VoteCache,
        VoteCacheProcessor, VoteGenerators, VoteProcessor, VoteProcessorExt, VoteProcessorQueue,
        VoteProcessorQueueCleanup, VoteRouter,
    },
    monitor::Monitor,
    node_id_key_file::NodeIdKeyFile,
    pruning::{LedgerPruning, LedgerPruningExt},
    representatives::{OnlineReps, OnlineRepsCleanup, RepCrawler, RepCrawlerExt},
    stats::{
        adapters::{LedgerStats, NetworkStats},
        DetailType, Direction, StatType, Stats,
    },
    transport::{
        keepalive::{KeepaliveMessageFactory, KeepalivePublisher},
        InboundMessageQueue, InboundMessageQueueCleanup, LatestKeepalives, LatestKeepalivesCleanup,
        MessageFlooder, MessageProcessor, MessagePublisher, NanoResponseServerSpawner,
        NetworkFilter, NetworkThreads, PeerCacheConnector, PeerCacheUpdater,
        RealtimeMessageHandler, SynCookies,
    },
    utils::{
        LongRunningTransactionLogger, ThreadPool, ThreadPoolImpl, TimerThread, TxnTrackingConfig,
    },
    wallets::{Wallets, WalletsExt},
    work::DistributedWorkFactory,
    NetworkParams, NodeCallbacks, OnlineWeightSampler, TelementryConfig, TelementryExt, Telemetry,
    BUILD_INFO, VERSION_STRING,
};
use rsnano_core::{
    utils::{system_time_as_nanoseconds, ContainerInfo, Peer},
    work::{WorkPool, WorkPoolImpl},
    Account, Amount, Block, BlockHash, BlockType, Networks, NodeId, PrivateKey, Root, SavedBlock,
    VoteCode, VoteSource,
};
use rsnano_ledger::{BlockStatus, Ledger, RepWeightCache};
use rsnano_messages::{ConfirmAck, Message, Publish};
use rsnano_network::{
    ChannelId, DeadChannelCleanup, DropPolicy, Network, NetworkCleanup, NetworkInfo, PeerConnector,
    TcpListener, TcpListenerExt, TrafficType,
};
use rsnano_nullable_clock::{SteadyClock, SystemTimeFactory};
use rsnano_nullable_http_client::{HttpClient, Url};
use rsnano_output_tracker::OutputListenerMt;
use rsnano_store_lmdb::{
    EnvOptions, LmdbConfig, LmdbEnv, LmdbStore, NullTransactionTracker, SyncStrategy,
    TransactionTracker,
};
use serde::Serialize;
use std::{
    collections::{HashMap, VecDeque},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, RwLock,
    },
    time::{Duration, SystemTime},
};
use tracing::{debug, error, info, warn};

pub struct Node {
    is_nulled: bool,
    pub runtime: tokio::runtime::Handle,
    pub data_path: PathBuf,
    pub steady_clock: Arc<SteadyClock>,
    pub node_id: PrivateKey,
    pub config: NodeConfig,
    pub network_params: NetworkParams,
    pub stats: Arc<Stats>,
    pub workers: Arc<dyn ThreadPool>,
    wallet_workers: Arc<dyn ThreadPool>,
    election_workers: Arc<dyn ThreadPool>,
    pub flags: NodeFlags,
    pub work: Arc<WorkPoolImpl>,
    pub distributed_work: Arc<DistributedWorkFactory>,
    pub store: Arc<LmdbStore>,
    pub unchecked: Arc<UncheckedMap>,
    pub ledger: Arc<Ledger>,
    pub syn_cookies: Arc<SynCookies>,
    pub network_info: Arc<RwLock<NetworkInfo>>,
    pub network: Arc<Network>,
    pub telemetry: Arc<Telemetry>,
    pub bootstrap_server: Arc<BootstrapServer>,
    online_weight_sampler: Arc<OnlineWeightSampler>,
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
    pub rep_crawler: Arc<RepCrawler>,
    pub tcp_listener: Arc<TcpListener>,
    pub election_schedulers: Arc<ElectionSchedulers>,
    pub request_aggregator: Arc<RequestAggregator>,
    pub backlog_population: Arc<BacklogPopulation>,
    pub bootstrap: Arc<BootstrapService>,
    pub local_block_broadcaster: Arc<LocalBlockBroadcaster>,
    pub process_live_dispatcher: Arc<ProcessLiveDispatcher>,
    message_processor: Mutex<MessageProcessor>,
    network_threads: Arc<Mutex<NetworkThreads>>,
    ledger_pruning: Arc<LedgerPruning>,
    pub peer_connector: Arc<PeerConnector>,
    peer_cache_updater: TimerThread<PeerCacheUpdater>,
    peer_cache_connector: TimerThread<PeerCacheConnector>,
    pub inbound_message_queue: Arc<InboundMessageQueue>,
    monitor: TimerThread<Monitor>,
    stopped: AtomicBool,
    pub network_filter: Arc<NetworkFilter>,
    pub message_publisher: Arc<Mutex<MessagePublisher>>, // TODO remove this. It is needed right now
    pub message_flooder: Arc<Mutex<MessageFlooder>>,     // TODO remove this. It is needed right now
    pub keepalive_publisher: Arc<KeepalivePublisher>,
    // to keep the weak pointer alive
    start_stop_listener: OutputListenerMt<&'static str>,
}

pub(crate) struct NodeArgs {
    pub runtime: tokio::runtime::Handle,
    pub data_path: PathBuf,
    pub config: NodeConfig,
    pub network_params: NetworkParams,
    pub flags: NodeFlags,
    pub work: Arc<WorkPoolImpl>,
    pub callbacks: NodeCallbacks,
}

impl NodeArgs {
    pub fn create_test_instance() -> Self {
        let network_params = NetworkParams::new(Networks::NanoDevNetwork);
        let config = NodeConfig::new(None, &network_params, 2);
        Self {
            runtime: tokio::runtime::Handle::current(),
            data_path: "/home/nulled-node".into(),
            network_params,
            config,
            flags: Default::default(),
            callbacks: Default::default(),
            work: Arc::new(WorkPoolImpl::new_null(123)),
        }
    }
}

impl Node {
    pub fn new_null() -> Self {
        Self::new_null_with_callbacks(Default::default())
    }

    pub fn new_null_with_callbacks(callbacks: NodeCallbacks) -> Self {
        let args = NodeArgs {
            callbacks,
            ..NodeArgs::create_test_instance()
        };
        Self::new(args, true, NodeIdKeyFile::new_null())
    }

    pub(crate) fn new_with_args(args: NodeArgs) -> Self {
        Self::new(args, false, NodeIdKeyFile::default())
    }

    pub fn node_id(&self) -> NodeId {
        self.node_id.public_key().into()
    }

    fn new(args: NodeArgs, is_nulled: bool, mut node_id_key_file: NodeIdKeyFile) -> Self {
        let network_params = args.network_params;
        let config = args.config;
        let flags = args.flags;
        let runtime = args.runtime;
        let work = args.work;
        // Time relative to the start of the node. This makes time exlicit and enables us to
        // write time relevant unit tests with ease.
        let steady_clock = Arc::new(SteadyClock::default());

        let network_label = network_params.network.get_current_network_as_string();
        let global_config = GlobalConfig {
            node_config: config.clone(),
            flags: flags.clone(),
            network_params: network_params.clone(),
        };
        let global_config = &global_config;
        let application_path = args.data_path;
        let node_id = node_id_key_file.initialize(&application_path).unwrap();

        let stats = Arc::new(Stats::new(config.stat_config.clone()));

        let store = if is_nulled {
            Arc::new(LmdbStore::new_null())
        } else {
            make_store(
                &application_path,
                true,
                &config.diagnostics_config.txn_tracking,
                Duration::from_millis(config.block_processor_batch_max_time_ms as u64),
                config.lmdb_config.clone(),
                config.backup_before_upgrade,
            )
            .expect("Could not create LMDB store")
        };

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
        info!("Node ID: {}", NodeId::from(&node_id));

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

        let network_info = Arc::new(RwLock::new(NetworkInfo::new(global_config.into())));

        let network_observer = Arc::new(NetworkStats::new(stats.clone()));

        let mut dead_channel_cleanup = DeadChannelCleanup::new(
            steady_clock.clone(),
            network_info.clone(),
            network_params.network.cleanup_cutoff(),
        );

        let mut network_filter = NetworkFilter::new(1024 * 1024);
        network_filter.age_cutoff = config.network_duplicate_filter_cutoff;
        let network_filter = Arc::new(network_filter);

        // empty `config.peering_port` means the user made no port choice at all;
        // otherwise, any value is considered, with `0` having the special meaning of 'let the OS pick a port instead'
        let mut network = Network::new(
            global_config.into(),
            network_info.clone(),
            steady_clock.clone(),
            runtime.clone(),
        );
        network.set_observer(network_observer.clone());
        let network = Arc::new(network);

        dead_channel_cleanup.add_step(NetworkCleanup::new(network.clone()));

        let mut inbound_message_queue =
            InboundMessageQueue::new(config.message_processor.max_queue, stats.clone());
        if let Some(cb) = args.callbacks.on_inbound {
            inbound_message_queue.set_inbound_callback(cb);
        }
        if let Some(cb) = args.callbacks.on_inbound_dropped {
            inbound_message_queue.set_inbound_dropped_callback(cb);
        }
        let inbound_message_queue = Arc::new(inbound_message_queue);

        dead_channel_cleanup.add_step(InboundMessageQueueCleanup::new(
            inbound_message_queue.clone(),
        ));

        let telemetry_config = TelementryConfig {
            enable_ongoing_requests: false,
            enable_ongoing_broadcasts: !flags.disable_providing_telemetry_metrics,
        };

        let unchecked = Arc::new(UncheckedMap::new(
            config.max_unchecked_blocks as usize,
            stats.clone(),
            flags.disable_block_processor_unchecked_deletion,
        ));

        let online_weight_sampler = Arc::new(OnlineWeightSampler::new(
            ledger.clone(),
            network_params.node.max_weight_samples as usize,
        ));

        let online_reps = Arc::new(Mutex::new(
            OnlineReps::builder()
                .rep_weights(rep_weights.clone())
                .weight_period(Duration::from_secs(network_params.node.weight_period))
                .online_weight_minimum(config.online_weight_minimum)
                .trended(online_weight_sampler.calculate_trend())
                .finish(),
        ));
        dead_channel_cleanup.add_step(OnlineRepsCleanup::new(online_reps.clone()));

        let mut message_publisher = MessagePublisher::new(
            network.clone(),
            stats.clone(),
            network_params.network.protocol_info(),
        );

        if let Some(callback) = &args.callbacks.on_publish {
            message_publisher.set_published_callback(callback.clone());
        }

        let message_flooder = MessageFlooder::new(
            online_reps.clone(),
            network.clone(),
            stats.clone(),
            message_publisher.clone(),
        );

        let telemetry = Arc::new(Telemetry::new(
            telemetry_config,
            config.clone(),
            stats.clone(),
            ledger.clone(),
            unchecked.clone(),
            network_params.clone(),
            network_info.clone(),
            message_publisher.clone(),
            node_id.clone(),
            steady_clock.clone(),
        ));

        let bootstrap_server = Arc::new(BootstrapServer::new(
            config.bootstrap_server.clone(),
            stats.clone(),
            ledger.clone(),
            message_publisher.clone(),
        ));
        dead_channel_cleanup.add_step(BootstrapServerCleanup::new(
            bootstrap_server.server_impl.clone(),
        ));

        let rep_tiers = Arc::new(RepTiers::new(
            rep_weights.clone(),
            network_params.clone(),
            online_reps.clone(),
            stats.clone(),
        ));

        let vote_processor_queue = Arc::new(VoteProcessorQueue::new(
            config.vote_processor.clone(),
            stats.clone(),
            rep_tiers.clone(),
        ));
        dead_channel_cleanup.add_step(VoteProcessorQueueCleanup::new(vote_processor_queue.clone()));

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
        dead_channel_cleanup.add_step(BlockProcessorCleanup::new(
            block_processor.processor_loop.clone(),
        ));

        let distributed_work = Arc::new(DistributedWorkFactory::new(work.clone(), runtime.clone()));

        let mut wallets_path = application_path.clone();
        wallets_path.push("wallets.ldb");

        let mut wallets_lmdb_config = config.lmdb_config.clone();
        wallets_lmdb_config.sync = SyncStrategy::Always;
        wallets_lmdb_config.map_size = 1024 * 1024 * 1024;
        let wallets_options = EnvOptions {
            config: wallets_lmdb_config,
            use_no_mem_init: false,
        };
        let wallets_env = if is_nulled {
            Arc::new(LmdbEnv::new_null())
        } else {
            Arc::new(LmdbEnv::new_with_options(wallets_path, &wallets_options).unwrap())
        };

        let mut wallets = Wallets::new(
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
            confirming_set.clone(),
            message_flooder.clone(),
        );
        if !is_nulled {
            wallets.initialize().expect("Could not create wallet");
        }
        let wallets = Arc::new(wallets);
        if !is_nulled {
            wallets.initialize2();
        }

        let vote_broadcaster = Arc::new(VoteBroadcaster::new(
            vote_processor_queue.clone(),
            message_flooder.clone(),
        ));

        let vote_generators = Arc::new(VoteGenerators::new(
            ledger.clone(),
            wallets.clone(),
            history.clone(),
            stats.clone(),
            &config,
            &network_params,
            vote_broadcaster,
            message_publisher.clone(),
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
            vote_cache.clone(),
            recently_confirmed.clone(),
            vote_applier.clone(),
            rep_weights.clone(),
        ));

        let on_vote = args
            .callbacks
            .on_vote
            .unwrap_or_else(|| Box::new(|_, _, _, _| {}));

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
            network_filter.clone(),
            network_info.clone(),
            vote_cache.clone(),
            stats.clone(),
            online_reps.clone(),
            flags.clone(),
            recently_confirmed,
            vote_applier.clone(),
            vote_router.clone(),
            vote_cache_processor.clone(),
            steady_clock.clone(),
            message_flooder.clone(),
        ));

        active_elections.initialize();

        let election_schedulers = Arc::new(ElectionSchedulers::new(
            config.clone(),
            network_params.network.clone(),
            active_elections.clone(),
            ledger.clone(),
            stats.clone(),
            vote_cache.clone(),
            confirming_set.clone(),
            online_reps.clone(),
        ));

        let schedulers_w = Arc::downgrade(&election_schedulers);
        let ledger_l = ledger.clone();
        // Activate accounts with fresh blocks
        block_processor.on_batch_processed(Box::new(move |batch| {
            let Some(schedulers) = schedulers_w.upgrade() else {
                return;
            };

            let tx = ledger_l.read_txn();
            for (status, context) in batch {
                if *status == BlockStatus::Progress {
                    let account = context
                        .saved_block
                        .lock()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .account();
                    schedulers.activate(&tx, &account);
                }
            }
        }));

        let schedulers_w = Arc::downgrade(&election_schedulers);
        active_elections.on_vacancy_updated(Box::new(move || {
            if let Some(schedulers) = schedulers_w.upgrade() {
                schedulers.notify();
            }
        }));

        if !flags.disable_activate_successors {
            let ledger_l = ledger.clone();
            let schedulers_w = Arc::downgrade(&election_schedulers);
            // Activate successors of cemented blocks
            confirming_set.on_batch_cemented(Box::new(move |batch| {
                let Some(schedulers) = schedulers_w.upgrade() else {
                    return;
                };
                let tx = ledger_l.read_txn();
                for context in batch {
                    schedulers.activate_successors(&tx, &context.block);
                }
            }));
        }

        vote_applier.set_election_schedulers(&election_schedulers);

        let process_live_dispatcher = Arc::new(ProcessLiveDispatcher::new());

        let mut bootstrap_publisher = MessagePublisher::new_with_buffer_size(
            network.clone(),
            stats.clone(),
            network_params.network.protocol_info(),
            512,
        );

        if let Some(callback) = &args.callbacks.on_publish {
            bootstrap_publisher.set_published_callback(callback.clone());
        }

        let latest_keepalives = Arc::new(Mutex::new(LatestKeepalives::default()));
        dead_channel_cleanup.add_step(LatestKeepalivesCleanup::new(latest_keepalives.clone()));

        let response_server_spawner = Arc::new(NanoResponseServerSpawner {
            tokio: runtime.clone(),
            stats: stats.clone(),
            node_id: node_id.clone(),
            network: network_info.clone(),
            inbound_queue: inbound_message_queue.clone(),
            node_flags: flags.clone(),
            network_params: network_params.clone(),
            syn_cookies: syn_cookies.clone(),
            latest_keepalives: latest_keepalives.clone(),
            network_filter: network_filter.clone(),
        });

        let peer_connector = Arc::new(PeerConnector::new(
            config.tcp.connect_timeout,
            network.clone(),
            network_observer.clone(),
            runtime.clone(),
            response_server_spawner.clone(),
            steady_clock.clone(),
        ));

        let keepalive_factory = Arc::new(KeepaliveMessageFactory::new(
            network_info.clone(),
            Peer::new(config.external_address.clone(), config.external_port),
        ));

        let keepalive_publisher = Arc::new(KeepalivePublisher::new(
            network_info.clone(),
            peer_connector.clone(),
            message_publisher.clone(),
            keepalive_factory.clone(),
        ));

        let rep_crawler = Arc::new(RepCrawler::new(
            online_reps.clone(),
            stats.clone(),
            config.rep_crawler_query_timeout,
            config.clone(),
            network_params.clone(),
            network_info.clone(),
            ledger.clone(),
            active_elections.clone(),
            steady_clock.clone(),
            message_publisher.clone(),
            keepalive_publisher.clone(),
            runtime.clone(),
        ));

        // BEWARE: `bootstrap` takes `network.port` instead of `config.peering_port` because when the user doesn't specify
        //         a peering port and wants the OS to pick one, the picking happens when `network` gets initialized
        //         (if UDP is active, otherwise it happens when `bootstrap` gets initialized), so then for TCP traffic
        //         we want to tell `bootstrap` to use the already picked port instead of itself picking a different one.
        //         Thus, be very careful if you change the order: if `bootstrap` gets constructed before `network`,
        //         the latter would inherit the port from the former (if TCP is active, otherwise `network` picks first)
        //
        let tcp_listener = Arc::new(TcpListener::new(
            network_info.read().unwrap().listening_port(),
            network.clone(),
            network_observer.clone(),
            runtime.clone(),
            response_server_spawner.clone(),
        ));

        let request_aggregator = Arc::new(RequestAggregator::new(
            config.request_aggregator.clone(),
            stats.clone(),
            vote_generators.clone(),
            ledger.clone(),
            network_info.clone(),
        ));
        dead_channel_cleanup.add_step(RequestAggregatorCleanup::new(
            request_aggregator.state.clone(),
        ));

        let backlog_population = Arc::new(BacklogPopulation::new(
            global_config.into(),
            ledger.clone(),
            stats.clone(),
            election_schedulers.clone(),
        ));

        let bootstrap = Arc::new(BootstrapService::new(
            block_processor.clone(),
            ledger.clone(),
            stats.clone(),
            network_info.clone(),
            message_publisher.clone(),
            global_config.node_config.bootstrap.clone(),
            steady_clock.clone(),
        ));

        let local_block_broadcaster = Arc::new(LocalBlockBroadcaster::new(
            config.local_block_broadcaster.clone(),
            block_processor.clone(),
            stats.clone(),
            ledger.clone(),
            confirming_set.clone(),
            message_flooder.clone(),
            !flags.disable_block_processor_republishing,
        ));
        local_block_broadcaster.initialize();

        let realtime_message_handler = Arc::new(RealtimeMessageHandler::new(
            stats.clone(),
            network_info.clone(),
            network_filter.clone(),
            block_processor.clone(),
            config.clone(),
            wallets.clone(),
            request_aggregator.clone(),
            vote_processor_queue.clone(),
            telemetry.clone(),
            bootstrap_server.clone(),
            bootstrap.clone(),
        ));

        let network_threads = Arc::new(Mutex::new(NetworkThreads::new(
            network_info.clone(),
            peer_connector.clone(),
            flags.clone(),
            network_params.clone(),
            stats.clone(),
            syn_cookies.clone(),
            network_filter.clone(),
            keepalive_factory.clone(),
            latest_keepalives.clone(),
            dead_channel_cleanup,
            message_flooder.clone(),
            steady_clock.clone(),
        )));

        let message_processor = Mutex::new(MessageProcessor::new(
            flags.clone(),
            config.clone(),
            inbound_message_queue.clone(),
            realtime_message_handler.clone(),
        ));

        debug!("Constructing node...");

        let schedulers_weak = Arc::downgrade(&election_schedulers);
        wallets.set_start_election_callback(Box::new(move |block| {
            if let Some(schedulers) = schedulers_weak.upgrade() {
                schedulers.add_manual(block);
            }
        }));

        let rep_crawler_w = Arc::downgrade(&rep_crawler);
        if !flags.disable_rep_crawler {
            network_info
                .write()
                .unwrap()
                .on_new_realtime_channel(Arc::new(move |channel| {
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
                    info.block.clone().into(),
                    BlockSource::Unchecked,
                    ChannelId::LOOPBACK,
                );
            }
        }));

        let wallets_w = Arc::downgrade(&wallets);
        let flooder_l = Mutex::new(message_flooder.clone());
        vote_router.on_vote_processed(Box::new(move |vote, _source, results| {
            let Some(wallets) = wallets_w.upgrade() else {
                return;
            };

            // Republish vote if it is new and the node does not host a principal representative (or close to)
            let processed = results.iter().any(|(_, code)| *code == VoteCode::Vote);
            if processed {
                if wallets.should_republish_vote(vote.voting_account.into()) {
                    let ack = Message::ConfirmAck(ConfirmAck::new_with_rebroadcasted_vote(
                        vote.as_ref().clone(),
                    ));
                    flooder_l
                        .lock()
                        .unwrap()
                        .flood(&ack, DropPolicy::CanDrop, 0.5);
                }
            }
        }));

        let keepalive_factory_w = Arc::downgrade(&keepalive_factory);
        let message_publisher_l = Arc::new(Mutex::new(message_publisher.clone()));
        let message_publisher_w = Arc::downgrade(&message_publisher_l);
        network_info
            .write()
            .unwrap()
            .on_new_realtime_channel(Arc::new(move |channel| {
                // Send a keepalive message to the new channel
                let Some(factory) = keepalive_factory_w.upgrade() else {
                    return;
                };
                let Some(publisher) = message_publisher_w.upgrade() else {
                    return;
                };
                let keepalive = factory.create_keepalive_self();
                publisher.lock().unwrap().try_send(
                    channel.channel_id(),
                    &keepalive,
                    DropPolicy::CanDrop,
                    TrafficType::Generic,
                );
            }));

        let rep_crawler_w = Arc::downgrade(&rep_crawler);
        let reps_w = Arc::downgrade(&online_reps);
        let clock = steady_clock.clone();
        vote_processor.on_vote_processed(Box::new(move |vote, channel_id, source, code| {
            debug_assert!(code != VoteCode::Invalid);
            let Some(rep_crawler) = rep_crawler_w.upgrade() else {
                return;
            };
            let Some(reps) = reps_w.upgrade() else {
                return;
            };
            // Ignore republished votes
            if source != VoteSource::Live {
                return;
            }

            let active_in_rep_crawler = rep_crawler.process(vote.clone(), channel_id);
            if active_in_rep_crawler {
                // Representative is defined as online if replying to live votes or rep_crawler queries
                reps.lock()
                    .unwrap()
                    .vote_observed(vote.voting_account, clock.now());
            }
        }));

        if !distributed_work.work_generation_enabled() {
            info!("Work generation is disabled");
        }

        info!(
            "Outbound bandwidth limit: {} bytes/s, burst ratio: {}",
            config.bandwidth_limit, config.bandwidth_limit_burst_ratio
        );

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
        confirming_set.on_cemented(Box::new(move |block| {
            let Some(workers) = workers_w.upgrade() else {
                return;
            };
            let Some(wallets) = wallets_w.upgrade() else {
                return;
            };

            // TODO: Is it neccessary to call this for all blocks?
            if block.is_send() {
                let block = block.clone();
                workers.post(Box::new(move || {
                    wallets.receive_confirmed(block.hash(), block.destination().unwrap())
                }));
            }
        }));

        if !config.callback_address.is_empty() {
            let tokio = runtime.clone();
            let stats = stats.clone();
            let url: Url = format!(
                "http://{}:{}{}",
                config.callback_address, config.callback_port, config.callback_target
            )
            .parse()
            .unwrap();
            active_elections.on_election_ended(Box::new(
                move |_txn,
                      status,
                      _weights,
                      account,
                      block,
                      amount,
                      is_state_send,
                      is_state_epoch| {
                    let block = block.clone();
                    if status.election_status_type == ElectionStatusType::ActiveConfirmedQuorum
                        || status.election_status_type
                            == ElectionStatusType::ActiveConfirmationHeight
                    {
                        let url = url.clone();
                        let stats = stats.clone();
                        tokio.spawn(async move {
                            let message = RpcCallbackMessage {
                                account: account.encode_account(),
                                hash: block.hash().encode_hex(),
                                block: (*block).clone().into(),
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
                                is_send: if is_state_send { Some("true") } else { None },
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
            network_info.clone(),
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
                network_info.clone(),
                online_reps.clone(),
                active_elections.clone(),
            ),
        );

        Self {
            is_nulled,
            steady_clock,
            peer_cache_updater: TimerThread::new("Peer history", peer_cache_updater),
            peer_cache_connector: TimerThread::new_run_immedately(
                "Net reachout",
                peer_cache_connector,
            ),
            peer_connector,
            node_id,
            workers,
            wallet_workers,
            election_workers,
            distributed_work,
            unchecked,
            telemetry,
            syn_cookies,
            network,
            network_info,
            ledger,
            store,
            stats,
            data_path: application_path,
            network_params,
            config,
            flags,
            work,
            runtime,
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
            rep_crawler,
            tcp_listener,
            election_schedulers,
            request_aggregator,
            backlog_population,
            bootstrap,
            local_block_broadcaster,
            process_live_dispatcher, // needs to stay alive
            ledger_pruning,
            network_threads,
            message_processor,
            inbound_message_queue,
            monitor,
            message_publisher: message_publisher_l,
            message_flooder: Arc::new(Mutex::new(message_flooder.clone())),
            network_filter,
            keepalive_publisher,
            stopped: AtomicBool::new(false),
            start_stop_listener: OutputListenerMt::new(),
        }
    }

    pub fn container_info(&self) -> ContainerInfo {
        let tcp_channels = self.network_info.read().unwrap().container_info();
        let online_reps = self.online_reps.lock().unwrap().container_info();
        let vote_cache = self.vote_cache.lock().unwrap().container_info();

        let network = ContainerInfo::builder()
            .node("tcp_channels", tcp_channels)
            .node("syn_cookies", self.syn_cookies.container_info())
            .finish();

        ContainerInfo::builder()
            .node("work", self.work.container_info())
            .node("ledger", self.ledger.container_info())
            .node("active", self.active.container_info())
            .node("network", network)
            .node("telemetry", self.telemetry.container_info())
            .node("wallets", self.wallets.container_info())
            .node("vote_processor", self.vote_processor_queue.container_info())
            .node(
                "vote_cache_processor",
                self.vote_cache_processor.container_info(),
            )
            .node("rep_crawler", self.rep_crawler.container_info())
            .node("block_processor", self.block_processor.container_info())
            .node("online_reps", online_reps)
            .node("history", self.history.container_info())
            .node("confirming_set", self.confirming_set.container_info())
            .node(
                "request_aggregator",
                self.request_aggregator.container_info(),
            )
            .node(
                "election_scheduler",
                self.election_schedulers.container_info(),
            )
            .node("vote_cache", vote_cache)
            .node("vote_router", self.vote_router.container_info())
            .node("vote_generators", self.vote_generators.container_info())
            .node("bootstrap_ascending", self.bootstrap.container_info())
            .node("unchecked", self.unchecked.container_info())
            .node(
                "local_block_broadcaster",
                self.local_block_broadcaster.container_info(),
            )
            .node("rep_tiers", self.rep_tiers.container_info())
            .node(
                "message_processor",
                self.inbound_message_queue.container_info(),
            )
            .finish()
    }

    fn long_inactivity_cleanup(&self) {
        let mut perform_cleanup = false;
        let mut tx = self.ledger.rw_txn();
        if self.ledger.store.online_weight.count(&tx) > 0 {
            let (sample_time, _) = self
                .ledger
                .store
                .online_weight
                .iter_rev(&tx)
                .next()
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

    pub fn process_local(&self, block: Block) -> Option<BlockStatus> {
        let result = self
            .block_processor
            .add_blocking(Arc::new(block), BlockSource::Local)
            .ok()?;
        match result {
            Ok(_) => Some(BlockStatus::Progress),
            Err(status) => Some(status),
        }
    }

    pub fn process(&self, block: Block) -> Result<SavedBlock, BlockStatus> {
        let mut tx = self.ledger.rw_txn();
        self.ledger.process(&mut tx, &block)
    }

    pub fn process_multi(&self, blocks: &[Block]) {
        let mut tx = self.ledger.rw_txn();
        for (i, block) in blocks.iter().enumerate() {
            self.ledger
                .process(&mut tx, &mut block.clone())
                .map_err(|e| anyhow!("Could not multi-process block index {}: {:?}", i, e))
                .unwrap();
        }
    }

    pub fn process_and_confirm_multi(&self, blocks: &[Block]) {
        self.process_multi(blocks);
        self.confirm_multi(blocks);
    }

    pub fn insert_into_wallet(&self, keys: &PrivateKey) {
        let wallet_id = self.wallets.wallet_ids()[0];
        self.wallets
            .insert_adhoc2(&wallet_id, &keys.raw_key(), true)
            .unwrap();
    }

    pub fn process_active(&self, block: Block) {
        self.block_processor.process_active(block);
    }

    pub fn process_local_multi(&self, blocks: &[Block]) {
        for block in blocks {
            let status = self.process_local(block.clone()).unwrap();
            if !matches!(status, BlockStatus::Progress | BlockStatus::Old) {
                panic!("could not process block!");
            }
        }
    }

    pub fn block(&self, hash: &BlockHash) -> Option<SavedBlock> {
        let tx = self.ledger.read_txn();
        self.ledger.any().get_block(&tx, hash)
    }

    pub fn latest(&self, account: &Account) -> BlockHash {
        let tx = self.ledger.read_txn();
        self.ledger
            .any()
            .account_head(&tx, account)
            .unwrap_or_default()
    }

    pub fn get_node_id(&self) -> NodeId {
        self.node_id.public_key().into()
    }

    pub fn work_generate_dev(&self, root: impl Into<Root>) -> u64 {
        self.work.generate_dev2(root.into()).unwrap()
    }

    pub fn block_exists(&self, hash: &BlockHash) -> bool {
        let tx = self.ledger.read_txn();
        self.ledger.any().block_exists(&tx, hash)
    }

    pub fn blocks_exist(&self, hashes: &[Block]) -> bool {
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

    pub fn confirm_multi(&self, blocks: &[Block]) {
        for block in blocks {
            self.confirm(block.hash());
        }
    }

    pub fn confirm(&self, hash: BlockHash) {
        let mut tx = self.ledger.rw_txn();
        self.ledger.confirm(&mut tx, hash);
    }

    pub fn block_confirmed(&self, hash: &BlockHash) -> bool {
        let tx = self.ledger.read_txn();
        self.ledger.confirmed().block_exists(&tx, hash)
    }

    pub fn block_hashes_confirmed(&self, blocks: &[BlockHash]) -> bool {
        let tx = self.ledger.read_txn();
        blocks
            .iter()
            .all(|b| self.ledger.confirmed().block_exists(&tx, b))
    }

    pub fn blocks_confirmed(&self, blocks: &[Block]) -> bool {
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
    fn flood_block_many(
        &self,
        blocks: VecDeque<Block>,
        callback: Box<dyn FnOnce() + Send + Sync>,
        delay: Duration,
    );
}

impl NodeExt for Arc<Node> {
    fn start(&self) {
        self.start_stop_listener.emit("start");
        if self.is_nulled {
            return; // TODO better nullability implementation
        }

        if !self.ledger.any().block_exists_or_pruned(
            &self.ledger.read_txn(),
            &self.network_params.ledger.genesis_block.hash(),
        ) {
            error!("Genesis block not found. This commonly indicates a configuration issue, check that the --network or --data_path command line arguments are correct, and also the ledger backend node config option. If using a read-only CLI command a ledger must already exist, start the node with --daemon first.");

            if self.network_params.network.is_beta_network() {
                error!("Beta network may have reset, try clearing database files");
            }

            panic!("Genesis block not found!");
        }

        self.long_inactivity_cleanup();
        self.network_threads.lock().unwrap().start();
        self.message_processor.lock().unwrap().start();

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

        self.unchecked.start();
        self.wallets.start();
        self.rep_tiers.start();
        if self.config.enable_vote_processor {
            self.vote_processor.start();
        }
        self.vote_cache_processor.start();
        self.block_processor.start();
        self.active.start();
        self.vote_generators.start();
        self.request_aggregator.start();
        self.confirming_set.start();
        self.election_schedulers.start();
        self.backlog_population.start();
        self.bootstrap_server.start();
        self.bootstrap
            .initialize(&self.network_params.ledger.genesis_account);
        self.bootstrap.start();
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

        if self.config.enable_monitor {
            self.monitor.start(self.config.monitor.interval);
        }
    }

    fn stop(&self) {
        self.start_stop_listener.emit("stop");
        if self.is_nulled {
            return; // TODO better nullability implementation
        }

        // Ensure stop can only be called once
        if self.stopped.swap(true, Ordering::SeqCst) {
            return;
        }
        info!("Node stopping...");

        self.tcp_listener.stop();
        self.vote_router.stop();
        self.peer_connector.stop();
        self.ledger_pruning.stop();
        self.peer_cache_connector.stop();
        self.peer_cache_updater.stop();
        // Cancels ongoing work generation tasks, which may be blocking other threads
        // No tasks may wait for work generation in I/O threads, or termination signal capturing will be unable to call node::stop()
        self.distributed_work.stop();
        self.backlog_population.stop();
        self.bootstrap.stop();
        self.rep_crawler.stop();
        self.unchecked.stop();
        self.block_processor.stop();
        self.request_aggregator.stop();
        self.vote_cache_processor.stop();
        self.vote_processor.stop();
        self.rep_tiers.stop();
        self.election_schedulers.stop();
        self.active.stop();
        self.vote_generators.stop();
        self.confirming_set.stop();
        self.telemetry.stop();
        self.bootstrap_server.stop();
        self.wallets.stop();
        self.stats.stop();
        self.local_block_broadcaster.stop();
        self.message_processor.lock().unwrap().stop();
        self.network_threads.lock().unwrap().stop(); // Stop network last to avoid killing in-use sockets
        self.monitor.stop();

        self.wallet_workers.stop();
        self.election_workers.stop();
        self.workers.stop();

        // work pool is not stopped on purpose due to testing setup
    }

    fn ongoing_online_weight_calculation_queue(&self) {
        let node_w = Arc::downgrade(self);
        self.workers.post_delayed(
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
        let mut backup_path = self.data_path.clone();
        backup_path.push("backup");
        if let Err(e) = self.wallets.backup(&backup_path) {
            error!(error = ?e, "Could not create backup of wallets");
        }

        let node_w = Arc::downgrade(self);
        self.workers.post_delayed(
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
        self.workers.post_delayed(
            Duration::from_secs(self.network_params.node.search_pending_interval_s as u64),
            Box::new(move || {
                if let Some(node) = node_w.upgrade() {
                    node.search_receivable_all();
                }
            }),
        )
    }

    fn flood_block_many(
        &self,
        mut blocks: VecDeque<Block>,
        callback: Box<dyn FnOnce() + Send + Sync>,
        delay: Duration,
    ) {
        if let Some(block) = blocks.pop_front() {
            let publish = Message::Publish(Publish::new_forward(block));
            self.message_flooder
                .lock()
                .unwrap()
                .flood(&publish, DropPolicy::CanDrop, 1.0);
            if blocks.is_empty() {
                callback()
            } else {
                let self_w = Arc::downgrade(self);
                self.workers.post_delayed(
                    delay,
                    Box::new(move || {
                        if let Some(node) = self_w.upgrade() {
                            node.flood_block_many(blocks, callback, delay);
                        }
                    }),
                );
            }
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
    use crate::{utils::TimerStartEvent, NodeBuilder};
    use rsnano_core::Networks;
    use std::ops::Deref;
    use uuid::Uuid;

    #[tokio::test]
    async fn start_peer_cache_updater() {
        let node = TestNode::new().await;
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

    #[tokio::test]
    async fn start_peer_cache_connector() {
        let node = TestNode::new().await;
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

    #[tokio::test]
    async fn stop_node() {
        let node = TestNode::new().await;
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
        pub async fn new() -> Self {
            let mut app_path = std::env::temp_dir();
            app_path.push(format!("rsnano-test-{}", Uuid::new_v4().simple()));
            let config = NodeConfig::new_test_instance();
            let network_params = NetworkParams::new(Networks::NanoDevNetwork);
            let work = Arc::new(WorkPoolImpl::new(
                network_params.work.clone(),
                1,
                Duration::ZERO,
            ));

            let node = NodeBuilder::new(Networks::NanoDevNetwork)
                .data_path(app_path.clone())
                .config(config)
                .network_params(network_params)
                .work(work)
                .finish()
                .unwrap();

            let node = Arc::new(node);

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
