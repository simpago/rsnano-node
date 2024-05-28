use crate::{
    block_processing::{
        BacklogPopulation, BacklogPopulationConfig, BlockProcessor, BlockSource,
        LocalBlockBroadcaster, LocalBlockBroadcasterExt, UncheckedMap,
    },
    bootstrap::{
        BootstrapAscending, BootstrapAscendingExt, BootstrapInitiator, BootstrapInitiatorExt,
        BootstrapMessageVisitorFactory, BootstrapServer,
    },
    cementation::ConfirmingSet,
    config::{FrontiersConfirmationMode, NodeConfig, NodeFlags},
    consensus::{
        AccountBalanceChangedCallback, ActiveTransactions, ActiveTransactionsExt, ElectionBehavior,
        ElectionEndCallback, ElectionStatusType, HintedScheduler, HintedSchedulerExt,
        LocalVoteHistory, ManualScheduler, ManualSchedulerExt, OptimisticScheduler,
        OptimisticSchedulerExt, PriorityScheduler, PrioritySchedulerExt, ProcessLiveDispatcher,
        ProcessLiveDispatcherExt, RepTiers, RequestAggregator, RequestAggregatorExt, VoteCache,
        VoteGenerator, VoteProcessor, VoteProcessorExt, VoteProcessorQueue,
    },
    node_id_key_file::NodeIdKeyFile,
    representatives::{RepCrawler, RepCrawlerExt, RepresentativeRegister},
    stats::{DetailType, Direction, LedgerStats, StatType, Stats},
    transport::{
        BufferDropPolicy, ChannelEnum, InboundCallback, KeepaliveFactory, LiveMessageProcessor,
        NetworkFilter, NetworkThreads, OutboundBandwidthLimiter, PeerCacheConnector,
        PeerCacheUpdater, SocketObserver, SynCookies, TcpChannels, TcpChannelsOptions, TcpListener,
        TcpListenerExt, TcpMessageManager, TrafficType,
    },
    utils::{
        AsyncRuntime, LongRunningTransactionLogger, ThreadPool, ThreadPoolImpl, TimerThread,
        TxnTrackingConfig,
    },
    wallets::{Wallets, WalletsExt},
    websocket::{create_websocket_server, WebsocketListenerExt},
    work::{DistributedWorkFactory, HttpClient},
    NetworkParams, OnlineReps, OnlineWeightSampler, TelementryConfig, TelementryExt, Telemetry,
    BUILD_INFO, VERSION_STRING,
};
use reqwest::Url;
use rsnano_core::{
    utils::{
        as_nano_json, system_time_as_nanoseconds, BufferReader, ContainerInfoComponent,
        Deserialize, SerdePropertyTree, StreamExt, SystemTimeFactory,
    },
    work::WorkPoolImpl,
    Account, Amount, BlockHash, BlockType, KeyPair, Networks, Vote, VoteCode,
};
use rsnano_ledger::{Ledger, Writer};
use rsnano_messages::{ConfirmAck, DeserializedMessage, Message};
use rsnano_store_lmdb::{
    EnvOptions, LmdbConfig, LmdbEnv, LmdbStore, NullTransactionTracker, SyncStrategy, Transaction,
    TransactionTracker,
};
use serde::Serialize;
use std::{
    borrow::Borrow,
    collections::{HashMap, VecDeque},
    net::{Ipv6Addr, SocketAddrV6},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc, Mutex, RwLock,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tracing::{debug, error, info, warn};

pub struct Node {
    pub async_rt: Arc<AsyncRuntime>,
    application_path: PathBuf,
    pub node_id: KeyPair,
    pub config: NodeConfig,
    network_params: NetworkParams,
    pub stats: Arc<Stats>,
    pub workers: Arc<dyn ThreadPool>,
    pub bootstrap_workers: Arc<dyn ThreadPool>,
    flags: NodeFlags,
    work: Arc<WorkPoolImpl>,
    pub distributed_work: Arc<DistributedWorkFactory>,
    pub store: Arc<LmdbStore>,
    pub unchecked: Arc<UncheckedMap>,
    pub ledger: Arc<Ledger>,
    pub outbound_limiter: Arc<OutboundBandwidthLimiter>,
    pub syn_cookies: Arc<SynCookies>,
    pub channels: Arc<TcpChannels>,
    pub telemetry: Arc<Telemetry>,
    pub bootstrap_server: Arc<BootstrapServer>,
    pub online_reps: Arc<Mutex<OnlineReps>>,
    pub online_reps_sampler: Arc<OnlineWeightSampler>,
    pub representative_register: Arc<Mutex<RepresentativeRegister>>,
    pub rep_tiers: Arc<RepTiers>,
    pub vote_processor_queue: Arc<VoteProcessorQueue>,
    pub history: Arc<LocalVoteHistory>,
    pub confirming_set: Arc<ConfirmingSet>,
    pub vote_cache: Arc<Mutex<VoteCache>>,
    pub block_processor: Arc<BlockProcessor>,
    pub wallets: Arc<Wallets>,
    pub vote_generator: Arc<VoteGenerator>,
    pub final_generator: Arc<VoteGenerator>,
    pub active: Arc<ActiveTransactions>,
    pub vote_processor: Arc<VoteProcessor>,
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
    pub live_message_processor: Arc<LiveMessageProcessor>,
    pub network_threads: Arc<Mutex<NetworkThreads>>,
    peer_cache_updater: TimerThread<PeerCacheUpdater>,
    peer_cache_connector: TimerThread<PeerCacheConnector>,
    warmed_up: AtomicU32,
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
        on_vote: Box<dyn Fn(&Arc<Vote>, &Arc<ChannelEnum>, VoteCode) + Send + Sync>,
    ) -> Self {
        let application_path = application_path.into();
        let node_id = NodeIdKeyFile::default()
            .initialize(&application_path)
            .unwrap();

        let stats = Arc::new(Stats::new(config.stat_config.clone()));

        let store = make_store(
            &application_path,
            true,
            &config.diagnostics_config.txn_tracking,
            Duration::from_millis(config.block_processor_batch_max_time_ms as u64),
            config.lmdb_config.clone(),
            config.backup_before_upgrade,
        )
        .expect("Could not create LMDB store");

        let mut ledger = Ledger::new(
            Arc::clone(&store),
            network_params.ledger.clone(),
            config.representative_vote_weight_minimum,
        )
        .expect("Could not initialize ledger");

        ledger.set_observer(Arc::new(LedgerStats::new(Arc::clone(&stats))));
        let ledger = Arc::new(ledger);

        let outbound_limiter = Arc::new(OutboundBandwidthLimiter::new(config.borrow().into()));
        let syn_cookies = Arc::new(SynCookies::new(network_params.network.max_peers_per_ip));
        let workers: Arc<dyn ThreadPool> = Arc::new(ThreadPoolImpl::create(
            config.background_threads as usize,
            "Worker".to_string(),
        ));

        // empty `config.peering_port` means the user made no port choice at all;
        // otherwise, any value is considered, with `0` having the special meaning of 'let the OS pick a port instead'
        let channels = Arc::new(TcpChannels::new(TcpChannelsOptions {
            node_config: config.clone(),
            publish_filter: Arc::new(NetworkFilter::new(256 * 1024)),
            async_rt: Arc::clone(&async_rt),
            network: network_params.clone(),
            stats: Arc::clone(&stats),
            tcp_message_manager: Arc::new(TcpMessageManager::new(
                config.tcp_incoming_connections_max as usize,
            )),
            port: config.peering_port.unwrap_or(0),
            flags: flags.clone(),
            limiter: Arc::clone(&outbound_limiter),
            node_id: node_id.clone(),
            syn_cookies: Arc::clone(&syn_cookies),
            workers: Arc::clone(&workers),
            observer: Arc::clone(&socket_observer),
        }));

        let telemetry_config = TelementryConfig {
            enable_ongoing_requests: !flags.disable_ongoing_telemetry_requests,
            enable_ongoing_broadcasts: !flags.disable_providing_telemetry_metrics,
        };

        let unchecked = Arc::new(UncheckedMap::new(
            config.max_unchecked_blocks as usize,
            Arc::clone(&stats),
            flags.disable_block_processor_unchecked_deletion,
        ));

        let telemetry = Arc::new(Telemetry::new(
            telemetry_config,
            config.clone(),
            Arc::clone(&stats),
            Arc::clone(&ledger),
            Arc::clone(&unchecked),
            network_params.clone(),
            Arc::clone(&channels),
            node_id.clone(),
        ));

        let bootstrap_server = Arc::new(BootstrapServer::new(
            Arc::clone(&stats),
            Arc::clone(&ledger),
        ));

        let mut online_reps = OnlineReps::new(Arc::clone(&ledger));
        online_reps.set_weight_period(Duration::from_secs(network_params.node.weight_period));
        online_reps.set_online_weight_minimum(config.online_weight_minimum);

        let mut online_reps_sampler = OnlineWeightSampler::new(Arc::clone(&ledger));
        online_reps_sampler.set_online_weight_minimum(config.online_weight_minimum);
        online_reps_sampler.set_max_samples(network_params.node.max_weight_samples);
        let online_reps_sampler = Arc::new(online_reps_sampler);
        online_reps.set_trended(online_reps_sampler.calculate_trend());
        let online_reps = Arc::new(Mutex::new(online_reps));

        let representative_register = Arc::new(Mutex::new(RepresentativeRegister::new(
            Arc::clone(&ledger),
            Arc::clone(&online_reps),
            Arc::clone(&stats),
            network_params.network.protocol_info(),
        )));

        let rep_tiers = Arc::new(RepTiers::new(
            Arc::clone(&ledger),
            network_params.clone(),
            Arc::clone(&online_reps),
            Arc::clone(&stats),
        ));

        let vote_processor_queue = Arc::new(VoteProcessorQueue::new(
            flags.vote_processor_capacity,
            Arc::clone(&stats),
            Arc::clone(&online_reps),
            Arc::clone(&ledger),
            Arc::clone(&rep_tiers),
        ));

        let history = Arc::new(LocalVoteHistory::new(network_params.voting.max_cache));

        let confirming_set = Arc::new(ConfirmingSet::new(
            Arc::clone(&ledger),
            config.confirming_set_batch_time,
        ));

        let vote_cache = Arc::new(Mutex::new(VoteCache::new(
            config.vote_cache.clone(),
            Arc::clone(&stats),
        )));

        let block_processor = Arc::new(BlockProcessor::new(
            Arc::new(config.clone()),
            Arc::new(flags.clone()),
            Arc::clone(&ledger),
            Arc::clone(&unchecked),
            Arc::clone(&stats),
            Arc::new(network_params.work.clone()),
        ));

        let distributed_work = Arc::new(DistributedWorkFactory::new(
            Arc::clone(&work),
            Arc::clone(&async_rt),
        ));

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
                config.enable_voting,
                wallets_env,
                Arc::clone(&ledger),
                &config,
                network_params.kdf_work,
                network_params.work.clone(),
                Arc::clone(&distributed_work),
                network_params.clone(),
                Arc::clone(&workers),
                Arc::clone(&block_processor),
                Arc::clone(&online_reps),
                Arc::clone(&channels),
                Arc::clone(&confirming_set),
            )
            .expect("Could not create wallet"),
        );
        wallets.initialize2();

        let inbound_impl: Arc<
            RwLock<Box<dyn Fn(DeserializedMessage, Arc<ChannelEnum>) + Send + Sync>>,
        > = Arc::new(RwLock::new(Box::new(|_msg, _channel| {
            panic!("inbound callback not set");
        })));
        let inbound_impl_clone = Arc::clone(&inbound_impl);
        let inbound: InboundCallback =
            Arc::new(move |msg: DeserializedMessage, channel: Arc<ChannelEnum>| {
                let cb = inbound_impl_clone.read().unwrap();
                (*cb)(msg, channel);
            });

        let vote_generator = Arc::new(VoteGenerator::new(
            Arc::clone(&ledger),
            Arc::clone(&wallets),
            Arc::clone(&history),
            false, //none-final
            Arc::clone(&stats),
            Arc::clone(&representative_register),
            Arc::clone(&channels),
            Arc::clone(&vote_processor_queue),
            network_params.network.clone(),
            Arc::clone(&async_rt),
            node_id.public_key(),
            SocketAddrV6::new(Ipv6Addr::LOCALHOST, channels.port(), 0, 0),
            Arc::clone(&inbound),
            Duration::from_secs(network_params.voting.delay_s as u64),
            Duration::from_millis(config.vote_generator_delay_ms as u64),
            config.vote_generator_threshold as usize,
        ));

        let final_generator = Arc::new(VoteGenerator::new(
            Arc::clone(&ledger),
            Arc::clone(&wallets),
            Arc::clone(&history),
            true, //final
            Arc::clone(&stats),
            Arc::clone(&representative_register),
            Arc::clone(&channels),
            Arc::clone(&vote_processor_queue),
            network_params.network.clone(),
            Arc::clone(&async_rt),
            node_id.public_key(),
            SocketAddrV6::new(Ipv6Addr::LOCALHOST, channels.port(), 0, 0),
            Arc::clone(&inbound),
            Duration::from_secs(network_params.voting.delay_s as u64),
            Duration::from_millis(config.vote_generator_delay_ms as u64),
            config.vote_generator_threshold as usize,
        ));

        let active = Arc::new(ActiveTransactions::new(
            network_params.clone(),
            Arc::clone(&online_reps),
            Arc::clone(&wallets),
            config.clone(),
            Arc::clone(&ledger),
            Arc::clone(&confirming_set),
            Arc::clone(&workers),
            Arc::clone(&history),
            Arc::clone(&block_processor),
            Arc::clone(&vote_generator),
            Arc::clone(&final_generator),
            Arc::clone(&channels),
            Arc::clone(&vote_cache),
            Arc::clone(&stats),
            election_end,
            account_balance_changed,
            Arc::clone(&representative_register),
            flags.clone(),
        ));

        active.initialize();

        let vote_processor = Arc::new(VoteProcessor::new(
            Arc::clone(&vote_processor_queue),
            Arc::clone(&active),
            Arc::clone(&stats),
            on_vote,
        ));

        let websocket = create_websocket_server(
            config.websocket_config.clone(),
            Arc::clone(&wallets),
            Arc::clone(&async_rt),
            &active,
            &telemetry,
            &vote_processor,
        );

        let bootstrap_initiator = Arc::new(BootstrapInitiator::new(
            config.clone(),
            flags.clone(),
            Arc::clone(&channels),
            Arc::clone(&async_rt),
            Arc::clone(&workers),
            network_params.clone(),
            Arc::clone(&socket_observer),
            Arc::clone(&stats),
            Arc::clone(&outbound_limiter),
            Arc::clone(&block_processor),
            websocket.clone(),
            Arc::clone(&ledger),
        ));
        bootstrap_initiator.initialize();
        bootstrap_initiator.start();

        let rep_crawler = Arc::new(RepCrawler::new(
            Arc::clone(&representative_register),
            Arc::clone(&stats),
            config.rep_crawler_query_timeout,
            Arc::clone(&online_reps),
            config.clone(),
            network_params.clone(),
            Arc::clone(&channels),
            Arc::clone(&async_rt),
            Arc::clone(&ledger),
            Arc::clone(&active),
        ));

        // BEWARE: `bootstrap` takes `network.port` instead of `config.peering_port` because when the user doesn't specify
        //         a peering port and wants the OS to pick one, the picking happens when `network` gets initialized
        //         (if UDP is active, otherwise it happens when `bootstrap` gets initialized), so then for TCP traffic
        //         we want to tell `bootstrap` to use the already picked port instead of itself picking a different one.
        //         Thus, be very careful if you change the order: if `bootstrap` gets constructed before `network`,
        //         the latter would inherit the port from the former (if TCP is active, otherwise `network` picks first)
        //
        let tcp_listener = Arc::new(TcpListener::new(
            channels.port(),
            config.tcp_incoming_connections_max as usize,
            config.clone(),
            Arc::clone(&channels),
            Arc::clone(&syn_cookies),
            network_params.clone(),
            flags.clone(),
            Arc::clone(&async_rt),
            socket_observer,
            Arc::clone(&stats),
            Arc::clone(&workers),
            Arc::clone(&block_processor),
            Arc::clone(&bootstrap_initiator),
            Arc::clone(&ledger),
            Arc::new(node_id.clone()),
        ));

        let hinted_scheduler = Arc::new(HintedScheduler::new(
            config.hinted_scheduler.clone(),
            Arc::clone(&active),
            Arc::clone(&ledger),
            Arc::clone(&stats),
            Arc::clone(&vote_cache),
            Arc::clone(&confirming_set),
            Arc::clone(&online_reps),
        ));

        let manual_scheduler = Arc::new(ManualScheduler::new(
            Arc::clone(&stats),
            Arc::clone(&active),
        ));

        let optimistic_scheduler = Arc::new(OptimisticScheduler::new(
            config.optimistic_scheduler.clone(),
            Arc::clone(&stats),
            Arc::clone(&active),
            network_params.network.clone(),
            Arc::clone(&ledger),
            Arc::clone(&confirming_set),
        ));

        let priority_scheduler = Arc::new(PriorityScheduler::new(
            Arc::clone(&ledger),
            Arc::clone(&stats),
            Arc::clone(&active),
        ));

        let priority_clone = Arc::downgrade(&priority_scheduler);
        active.set_activate_successors_callback(Box::new(move |tx, block| {
            if let Some(priority) = priority_clone.upgrade() {
                priority.activate_successors(&tx, block);
            }
        }));

        let request_aggregator = Arc::new(RequestAggregator::new(
            config.clone(),
            Arc::clone(&stats),
            Arc::clone(&vote_generator),
            Arc::clone(&final_generator),
            Arc::clone(&history),
            Arc::clone(&ledger),
            Arc::clone(&wallets),
            Arc::clone(&active),
            network_params.network.is_dev_network(),
        ));
        request_aggregator.start();

        let backlog_population = Arc::new(BacklogPopulation::new(
            BacklogPopulationConfig {
                enabled: config.frontiers_confirmation != FrontiersConfirmationMode::Disabled,
                batch_size: config.backlog_scan_batch_size,
                frequency: config.backlog_scan_frequency,
            },
            Arc::clone(&ledger),
            Arc::clone(&stats),
        ));

        let ascendboot = Arc::new(BootstrapAscending::new(
            Arc::clone(&block_processor),
            Arc::clone(&ledger),
            Arc::clone(&stats),
            Arc::clone(&channels),
            config.clone(),
            network_params.network.clone(),
        ));

        let local_block_broadcaster = Arc::new(LocalBlockBroadcaster::new(
            Arc::clone(&block_processor),
            Arc::clone(&stats),
            Arc::clone(&channels),
            Arc::clone(&representative_register),
            Arc::clone(&ledger),
            Arc::clone(&confirming_set),
            !flags.disable_block_processor_republishing,
        ));
        local_block_broadcaster.initialize();

        let process_live_dispatcher = Arc::new(ProcessLiveDispatcher::new(
            Arc::clone(&ledger),
            Arc::clone(&priority_scheduler),
            websocket.clone(),
        ));

        let live_message_processor = Arc::new(LiveMessageProcessor::new(
            Arc::clone(&stats),
            Arc::clone(&channels),
            Arc::clone(&block_processor),
            config.clone(),
            flags.clone(),
            Arc::clone(&wallets),
            Arc::clone(&request_aggregator),
            Arc::clone(&vote_processor_queue),
            Arc::clone(&telemetry),
            Arc::clone(&bootstrap_server),
            Arc::clone(&ascendboot),
        ));

        let live_message_processor_weak = Arc::downgrade(&live_message_processor);
        *inbound_impl.write().unwrap() =
            Box::new(move |msg: DeserializedMessage, channel: Arc<ChannelEnum>| {
                if let Some(processor) = live_message_processor_weak.upgrade() {
                    processor.process(msg.message, &channel);
                }
            });

        let keepalive_factory = Arc::new(KeepaliveFactory {
            channels: Arc::clone(&channels),
            config: config.clone(),
        });
        let network_threads = Arc::new(Mutex::new(NetworkThreads::new(
            Arc::clone(&channels),
            config.clone(),
            flags.clone(),
            network_params.clone(),
            Arc::clone(&stats),
            Arc::clone(&syn_cookies),
            Arc::clone(&keepalive_factory),
        )));

        let processor = Arc::downgrade(&live_message_processor);
        channels.set_sink(Box::new(move |msg, channel| {
            if let Some(processor) = processor.upgrade() {
                processor.process(msg.message, &channel);
            }
        }));

        debug!("Constructing node...");

        let manual_weak = Arc::downgrade(&manual_scheduler);
        wallets.set_start_election_callback(Box::new(move |block| {
            if let Some(manual) = manual_weak.upgrade() {
                manual.push(block, None, ElectionBehavior::Normal);
            }
        }));

        let rep_crawler_w = Arc::downgrade(&rep_crawler);
        if !flags.disable_rep_crawler {
            channels.on_new_channel(Arc::new(move |channel| {
                if let Some(crawler) = rep_crawler_w.upgrade() {
                    crawler.query_channel(channel);
                }
            }));
        }

        let block_processor_w = Arc::downgrade(&block_processor);
        let history_w = Arc::downgrade(&history);
        let active_w = Arc::downgrade(&active);
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

        channels.set_observer(Arc::downgrade(&Arc::clone(&tcp_listener).as_observer()));

        let bootstrap_workers: Arc<dyn ThreadPool> = Arc::new(ThreadPoolImpl::create(
            config.bootstrap_serving_threads as usize,
            "Bootstrap work".to_string(),
        ));

        let visitor_factory = Arc::new(BootstrapMessageVisitorFactory::new(
            Arc::clone(&async_rt),
            Arc::clone(&syn_cookies),
            Arc::clone(&stats),
            network_params.network.clone(),
            Arc::new(node_id.clone()),
            Arc::clone(&ledger),
            Arc::clone(&bootstrap_workers),
            Arc::clone(&block_processor),
            Arc::clone(&bootstrap_initiator),
            flags.clone(),
        ));

        channels.set_message_visitor_factory(visitor_factory);

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

        let priority_w = Arc::downgrade(&priority_scheduler);
        let optimistic_w = Arc::downgrade(&optimistic_scheduler);
        backlog_population.set_activate_callback(Box::new(move |tx, account, info, height| {
            let Some(priority) = priority_w.upgrade() else {
                return;
            };
            let Some(optimistic) = optimistic_w.upgrade() else {
                return;
            };
            priority.activate(account, tx);
            optimistic.activate(*account, info, height);
        }));

        let ledger_w = Arc::downgrade(&ledger);
        let vote_cache_w = Arc::downgrade(&vote_cache);
        let wallets_w = Arc::downgrade(&wallets);
        let channels_w = Arc::downgrade(&channels);
        active.add_vote_processed_observer(Box::new(move |vote, source, results| {
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
            vote_cache
                .lock()
                .unwrap()
                .observe(vote, rep_weight, source, results.clone());

            // Republish vote if it is new and the node does not host a principal representative (or close to)
            let processed = results.iter().any(|(_, code)| *code == VoteCode::Vote);
            if processed {
                if wallets.should_republish_vote(vote.voting_account) {
                    let ack = Message::ConfirmAck(ConfirmAck::new(vote.as_ref().clone()));
                    channels.flood_message(&ack, 0.5);
                }
            }
        }));

        let priority_w = Arc::downgrade(&priority_scheduler);
        let hinted_w = Arc::downgrade(&hinted_scheduler);
        let optimistic_w = Arc::downgrade(&optimistic_scheduler);
        // Notify election schedulers when AEC frees election slot
        *active.vacancy_update.lock().unwrap() = Box::new(move || {
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
        channels.on_new_channel(Arc::new(move |channel| {
            let Some(factory) = keepalive_factory_w.upgrade() else {
                return;
            };
            let keepalive = factory.create_keepalive_self();
            let msg = Message::Keepalive(keepalive);
            channel.send(&msg, None, BufferDropPolicy::Limiter, TrafficType::Generic);
        }));

        // Add block confirmation type stats regardless of http-callback and websocket subscriptions
        let stats_w = Arc::downgrade(&stats);
        active.add_election_end_callback(Box::new(
            move |status, _votes, _account, _amount, _is_state_send, _is_state_epoch| {
                let Some(stats) = stats_w.upgrade() else {
                    return;
                };
                match status.election_status_type {
                    ElectionStatusType::ActiveConfirmedQuorum => stats.inc_dir(
                        StatType::ConfirmationObserver,
                        DetailType::ActiveQuorum,
                        Direction::Out,
                    ),
                    ElectionStatusType::ActiveConfirmationHeight => stats.inc_dir(
                        StatType::ConfirmationObserver,
                        DetailType::ActiveConfHeight,
                        Direction::Out,
                    ),
                    ElectionStatusType::InactiveConfirmationHeight => stats.inc_dir(
                        StatType::ConfirmationObserver,
                        DetailType::InactiveConfHeight,
                        Direction::Out,
                    ),
                    ElectionStatusType::Ongoing => unreachable!(),
                    ElectionStatusType::Stopped => {}
                }
            },
        ));

        let rep_crawler_w = Arc::downgrade(&rep_crawler);
        let online_reps_w = Arc::downgrade(&online_reps);
        vote_processor.add_vote_processed_callback(Box::new(move |vote, channel, code| {
            debug_assert!(code != VoteCode::Invalid);
            let Some(rep_crawler) = rep_crawler_w.upgrade() else {
                return;
            };
            let Some(online_reps) = online_reps_w.upgrade() else {
                return;
            };
            let active_in_rep_crawler = rep_crawler.process(Arc::clone(vote), Arc::clone(channel));
            if active_in_rep_crawler {
                // Representative is defined as online if replying to live votes or rep_crawler queries
                online_reps.lock().unwrap().observe(vote.voting_account);
            }
        }));

        let network_label = network_params.network.get_current_network_as_string();
        info!("Node starting, version: {}", VERSION_STRING);
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

        if !distributed_work.work_generation_enabled() {
            info!("Work generation is disabled");
        }

        info!(
            "Outbound bandwidth limit: {} bytes/s, burst ratio: {}",
            config.bandwidth_limit, config.bandwidth_limit_burst_ratio
        );

        if !ledger.block_or_pruned_exists(&network_params.ledger.genesis.hash()) {
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

        if (network_params.network.is_live_network() || network_params.network.is_beta_network())
            && !flags.inactive_node
        {
            let (max_blocks, weights) =
                get_bootstrap_weights(network_params.network.current_network);
            ledger.set_bootstrap_weight_max_blocks(max_blocks);

            info!(
                "Initial bootstrap height: {}",
                ledger.bootstrap_weight_max_blocks()
            );
            info!("Current ledger height:    {}", ledger.block_count());

            // Use bootstrap weights if initial bootstrap is not completed
            let use_bootstrap_weight = ledger.block_count() < max_blocks;
            if use_bootstrap_weight {
                info!("Using predefined representative weights, since block count is less than bootstrap threshold");
                *ledger.bootstrap_weights.lock().unwrap() = weights;

                info!("************************************ Bootstrap weights ************************************");
                // Sort the weights
                let mut sorted_weights = ledger
                    .bootstrap_weights
                    .lock()
                    .unwrap()
                    .iter()
                    .map(|(account, weight)| (*account, *weight))
                    .collect::<Vec<_>>();
                sorted_weights.sort_by(|(_, weight_a), (_, weight_b)| weight_b.cmp(weight_a));

                for (rep, weight) in sorted_weights {
                    info!(
                        "Using bootstrap rep weight: {} -> {}",
                        rep.encode_account(),
                        weight.format_balance(0)
                    );
                }
                info!("************************************ ================= ************************************");
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

        let workers_w = Arc::downgrade(&workers);
        let wallets_w = Arc::downgrade(&wallets);
        confirming_set.add_cemented_observer(Box::new(move |block| {
            let Some(workers) = workers_w.upgrade() else {
                return;
            };
            let Some(wallets) = wallets_w.upgrade() else {
                return;
            };

            if block.is_send() {
                let block = Arc::clone(block);
                workers.push_task(Box::new(move || {
                    wallets.receive_confirmed(block.hash(), block.destination().unwrap())
                }));
            }
        }));

        if !config.callback_address.is_empty() {
            let async_rt = Arc::clone(&async_rt);
            let stats = Arc::clone(&stats);
            let url: Url = format!(
                "http://{}:{}{}",
                config.callback_address, config.callback_port, config.callback_target
            )
            .parse()
            .unwrap();
            active.add_election_end_callback(Box::new(
                move |status, _weights, account, amount, is_state_send, is_state_epoch| {
                    let block = Arc::clone(status.winner.as_ref().unwrap());
                    if status.election_status_type == ElectionStatusType::ActiveConfirmedQuorum
                        || status.election_status_type
                            == ElectionStatusType::ActiveConfirmationHeight
                    {
                        let url = url.clone();
                        let stats = Arc::clone(&stats);
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
            Arc::clone(&channels),
            Arc::clone(&ledger),
            time_factory,
            Arc::clone(&stats),
            Duration::from_secs(60 * 60),
        );

        let peer_cache_connector = PeerCacheConnector::new(
            Arc::clone(&ledger),
            Arc::clone(&channels),
            Arc::clone(&stats),
            network_params.network.merge_period,
        );

        Self {
            peer_cache_updater: TimerThread::new(
                "Peer history",
                peer_cache_updater,
                Duration::from_secs(15),
            ),
            peer_cache_connector: TimerThread::new_run_immedately(
                "Net reachout",
                peer_cache_connector,
                network_params.network.merge_period,
            ),
            node_id,
            workers,
            bootstrap_workers,
            distributed_work,
            unchecked,
            telemetry,
            outbound_limiter,
            syn_cookies,
            channels,
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
            online_reps,
            online_reps_sampler,
            representative_register,
            rep_tiers,
            vote_processor_queue,
            history,
            confirming_set,
            vote_cache,
            block_processor,
            wallets,
            vote_generator,
            final_generator,
            active,
            vote_processor,
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
            live_message_processor,
            network_threads,
            warmed_up: AtomicU32::new(0),
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
                self.tcp_listener.collect_container_info("tcp_listener"),
                ContainerInfoComponent::Composite(
                    "network".to_string(),
                    vec![
                        self.channels.collect_container_info("tcp_channels"),
                        self.syn_cookies.collect_container_info("syn_cookies"),
                        self.channels
                            .excluded_peers
                            .lock()
                            .unwrap()
                            .collect_container_info("excluded_peers"),
                    ],
                ),
                self.telemetry.collect_container_info("telemetry"),
                self.wallets.collect_container_info("wallets"),
                self.vote_processor_queue
                    .collect_container_info("vote_processor"),
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
                self.vote_generator.collect_container_info("vote_generator"),
                self.final_generator
                    .collect_container_info("vote_generator_final"),
                self.ascendboot
                    .collect_container_info("bootstrap_ascending"),
                self.unchecked.collect_container_info("unchecked"),
                self.local_block_broadcaster
                    .collect_container_info("local_block_broadcaster"),
                self.rep_tiers.collect_container_info("rep_tiers"),
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

    fn collect_ledger_pruning_targets(
        &self,
        pruning_targets_a: &mut VecDeque<BlockHash>,
        last_account_a: &mut Account,
        batch_read_size_a: u64,
        max_depth_a: u64,
        cutoff_time_a: u64,
    ) -> bool {
        let mut read_operations = 0;
        let mut finish_transaction = false;
        let mut tx = self.ledger.read_txn();
        let mut it = self
            .ledger
            .store
            .confirmation_height
            .begin_at_account(&tx, &last_account_a);
        while let Some((&account, info)) = it.current() {
            if finish_transaction {
                break;
            }
            read_operations += 1;
            let mut hash = info.frontier;
            let mut depth = 0;
            while !hash.is_zero() && depth < max_depth_a {
                if let Some(block) = self.ledger.get_block(&tx, &hash) {
                    if block.sideband().unwrap().timestamp > cutoff_time_a || depth == 0 {
                        hash = block.previous();
                    } else {
                        break;
                    }
                } else {
                    assert!(depth != 0);
                    hash = BlockHash::zero();
                }
                depth += 1;
                if depth % batch_read_size_a == 0 {
                    drop(it);
                    tx.refresh();
                    it = self
                        .ledger
                        .store
                        .confirmation_height
                        .begin_at_account(&tx, &account);
                }
            }
            if !hash.is_zero() {
                pruning_targets_a.push_back(hash);
            }
            read_operations += depth;
            if read_operations >= batch_read_size_a {
                *last_account_a = account.inc().unwrap_or_default();
                finish_transaction = true;
            } else {
                it.next();
            }
        }

        !finish_transaction || last_account_a.is_zero()
    }

    pub fn is_stopped(&self) -> bool {
        self.stopped.load(Ordering::SeqCst)
    }
}

pub trait NodeExt {
    fn start(&self);
    fn stop(&self);
    fn ongoing_bootstrap(&self);
    fn ongoing_ledger_pruning(&self);
    fn ledger_pruning(&self, batch_size_a: u64, bootstrap_weight_reached_a: bool);
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
        if !self.flags.disable_legacy_bootstrap && !self.flags.disable_ongoing_bootstrap {
            self.ongoing_bootstrap();
        }

        if self.flags.enable_pruning {
            let node_w = Arc::downgrade(self);
            self.workers.push_task(Box::new(move || {
                if let Some(node) = node_w.upgrade() {
                    node.ongoing_ledger_pruning();
                }
            }));
        }

        if !self.flags.disable_rep_crawler {
            self.rep_crawler.start();
        }
        self.ongoing_online_weight_calculation_queue();

        if self.config.tcp_incoming_connections_max > 0
            && !(self.flags.disable_bootstrap_listener && self.flags.disable_tcp_realtime)
        {
            let listener_w = Arc::downgrade(&self.tcp_listener);
            self.tcp_listener
                .start_with(Box::new(move |new_connection, ec| {
                    let Some(listener_l) = listener_w.upgrade() else {
                        return false;
                    };
                    if ec.is_ok() {
                        listener_l.accept_action(ec, new_connection);
                    }
                    true
                }))
                .expect("could not start tcp listener");

            if self.channels.port() != self.tcp_listener.endpoint().port() {
                self.channels.set_port(self.tcp_listener.endpoint().port());
            }

            info!("Node peering port: {}", self.channels.port());
        } else {
            warn!("Node peering is disabled");
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
        self.block_processor.start();
        self.active.start();
        self.vote_generator.start();
        self.final_generator.start();
        self.confirming_set.start();
        self.hinted_scheduler.start();
        self.manual_scheduler.start();
        self.optimistic_scheduler.start();
        self.priority_scheduler.start();
        self.backlog_population.start();
        self.bootstrap_server.start();
        if !self.flags.disable_ascending_bootstrap {
            self.ascendboot.start();
        }
        if let Some(ws_listener) = &self.websocket {
            ws_listener.start();
        }
        self.telemetry.start();
        self.local_block_broadcaster.start();
        self.peer_cache_updater.start();
        self.peer_cache_connector.start();
    }

    fn stop(&self) {
        // Ensure stop can only be called once
        if self.stopped.swap(true, Ordering::SeqCst) {
            return;
        }
        info!("Node stopping...");

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
        self.vote_processor.stop();
        self.rep_tiers.stop();
        self.hinted_scheduler.stop();
        self.manual_scheduler.stop();
        self.optimistic_scheduler.stop();
        self.priority_scheduler.stop();
        self.active.stop();
        self.vote_generator.stop();
        self.final_generator.stop();
        self.confirming_set.stop();
        self.telemetry.stop();
        if let Some(ws_listener) = &self.websocket {
            ws_listener.stop();
        }
        self.bootstrap_server.stop();
        self.bootstrap_initiator.stop();
        self.tcp_listener.stop();
        self.wallets.stop();
        self.stats.stop();
        self.workers.stop();
        self.local_block_broadcaster.stop();
        self.network_threads.lock().unwrap().stop(); // Stop network last to avoid killing in-use sockets

        // work pool is not stopped on purpose due to testing setup
    }

    fn ongoing_bootstrap(&self) {
        let mut next_wakeup =
            Duration::from_secs(self.network_params.network.bootstrap_interval_s as u64);
        if self.warmed_up.load(Ordering::SeqCst) < 3 {
            // Re-attempt bootstrapping more aggressively on startup
            next_wakeup = Duration::from_secs(5);
            if !self.bootstrap_initiator.in_progress() && !self.channels.len() == 0 {
                self.warmed_up.fetch_add(1, Ordering::SeqCst);
            }
        }
        if self.network_params.network.is_dev_network() && self.flags.bootstrap_interval != 0 {
            // For test purposes allow faster automatic bootstraps
            next_wakeup = Duration::from_secs(self.flags.bootstrap_interval as u64);
            self.warmed_up.fetch_add(1, Ordering::SeqCst);
        }
        // Differential bootstrap with max age (75% of all legacy attempts)
        let mut frontiers_age = u32::MAX;
        let bootstrap_weight_reached =
            self.ledger.block_count() >= self.ledger.bootstrap_weight_max_blocks();
        let previous_bootstrap_count =
            self.stats
                .count(StatType::Bootstrap, DetailType::Initiate, Direction::Out)
                + self.stats.count(
                    StatType::Bootstrap,
                    DetailType::InitiateLegacyAge,
                    Direction::Out,
                );
        /*
        - Maximum value for 25% of attempts or if block count is below preconfigured value (initial bootstrap not finished)
        - Node shutdown time minus 1 hour for start attempts (warm up)
        - Default age value otherwise (1 day for live network, 1 hour for beta)
        */
        if bootstrap_weight_reached {
            if self.warmed_up.load(Ordering::SeqCst) < 3 {
                // Find last online weight sample (last active time for node)
                let mut last_sample_time = UNIX_EPOCH;

                {
                    let tx = self.ledger.read_txn();
                    let last_record = self.ledger.store.online_weight.rbegin(&tx);
                    if let Some(last_record) = last_record.current() {
                        last_sample_time = UNIX_EPOCH
                            .checked_add(Duration::from_nanos(*last_record.0))
                            .unwrap();
                    }
                }

                let time_since_last_sample = SystemTime::now()
                    .duration_since(last_sample_time)
                    .unwrap_or(Duration::MAX);

                if time_since_last_sample.as_secs() + 60 * 60 < u32::MAX as u64 {
                    frontiers_age = std::cmp::max(
                        (time_since_last_sample.as_secs() + 60 * 60) as u32,
                        self.network_params.bootstrap.default_frontiers_age_seconds,
                    );
                }
            } else if previous_bootstrap_count % 4 != 0 {
                frontiers_age = self.network_params.bootstrap.default_frontiers_age_seconds;
            }
        }
        // Bootstrap and schedule for next attempt
        self.bootstrap_initiator.bootstrap(
            false,
            format!("auto_bootstrap_{}", previous_bootstrap_count),
            frontiers_age,
            Account::zero(),
        );
        let self_w = Arc::downgrade(self);
        self.workers.add_delayed_task(
            next_wakeup,
            Box::new(move || {
                if let Some(node) = self_w.upgrade() {
                    node.ongoing_bootstrap();
                }
            }),
        );
    }

    fn ongoing_ledger_pruning(&self) {
        let bootstrap_weight_reached =
            self.ledger.block_count() >= self.ledger.bootstrap_weight_max_blocks();
        self.ledger_pruning(
            if self.flags.block_processor_batch_size != 0 {
                self.flags.block_processor_batch_size as u64
            } else {
                2 * 1024
            },
            bootstrap_weight_reached,
        );
        let ledger_pruning_interval = if bootstrap_weight_reached {
            Duration::from_secs(self.config.max_pruning_age_s as u64)
        } else {
            Duration::from_secs(std::cmp::min(self.config.max_pruning_age_s as u64, 15 * 60))
        };
        let node_w = Arc::downgrade(self);
        self.workers.add_delayed_task(
            ledger_pruning_interval,
            Box::new(move || {
                if let Some(node) = node_w.upgrade() {
                    node.ongoing_ledger_pruning()
                }
            }),
        );
    }

    fn ledger_pruning(&self, batch_size_a: u64, bootstrap_weight_reached_a: bool) {
        let max_depth = if self.config.max_pruning_depth != 0 {
            self.config.max_pruning_depth
        } else {
            u64::MAX
        };
        let cutoff_time = if bootstrap_weight_reached_a {
            (SystemTime::now() - Duration::from_secs(self.config.max_pruning_age_s as u64))
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
        } else {
            u64::MAX
        };
        let mut pruned_count = 0;
        let mut transaction_write_count = 0;
        let mut last_account = Account::from(1); // 0 Burn account is never opened. So it can be used to break loop
        let mut pruning_targets = VecDeque::new();
        let mut target_finished = false;
        while (transaction_write_count != 0 || !target_finished)
            && !self.stopped.load(Ordering::SeqCst)
        {
            // Search pruning targets
            while pruning_targets.len() < batch_size_a as usize
                && !target_finished
                && !self.stopped.load(Ordering::SeqCst)
            {
                target_finished = self.collect_ledger_pruning_targets(
                    &mut pruning_targets,
                    &mut last_account,
                    batch_size_a * 2,
                    max_depth,
                    cutoff_time,
                );
            }
            // Pruning write operation
            transaction_write_count = 0;
            if !pruning_targets.is_empty() && !self.stopped.load(Ordering::SeqCst) {
                let _scoped_write_guard = self.ledger.write_queue.wait(Writer::Pruning);
                let mut tx = self.ledger.rw_txn();
                while !pruning_targets.is_empty()
                    && transaction_write_count < batch_size_a
                    && !self.stopped.load(Ordering::SeqCst)
                {
                    let pruning_hash = pruning_targets.front().unwrap();
                    let account_pruned_count =
                        self.ledger
                            .pruning_action(&mut tx, pruning_hash, batch_size_a);
                    transaction_write_count += account_pruned_count;
                    pruning_targets.pop_front();
                }
                pruned_count += transaction_write_count;

                debug!("Pruned blocks: {}", pruned_count);
            }
        }

        debug!("Total recently pruned block count: {}", pruned_count);
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
        let online = self.online_reps.lock().unwrap().online();
        self.online_reps_sampler.sample(online);
        let trend = self.online_reps_sampler.calculate_trend();
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

fn get_bootstrap_weights(network: Networks) -> (u64, HashMap<Account, Amount>) {
    let buffer = get_bootstrap_weights_bin(network);
    deserialize_bootstrap_weights(buffer)
}

fn get_bootstrap_weights_bin(network: Networks) -> &'static [u8] {
    if network == Networks::NanoLiveNetwork {
        include_bytes!("../../../rep_weights_live.bin")
    } else {
        include_bytes!("../../../rep_weights_beta.bin")
    }
}

fn deserialize_bootstrap_weights(buffer: &[u8]) -> (u64, HashMap<Account, Amount>) {
    let mut reader = BufferReader::new(buffer);
    let mut weights = HashMap::new();
    let mut max_blocks = 0;
    if let Ok(count) = reader.read_u128_be() {
        max_blocks = count as u64;
        loop {
            let Ok(account) = Account::deserialize(&mut reader) else {
                break;
            };
            let Ok(weight) = Amount::deserialize(&mut reader) else {
                break;
            };
            weights.insert(account, weight);
        }
    }

    (max_blocks, weights)
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
    use std::ops::Deref;
    use uuid::Uuid;

    #[test]
    fn bootstrap_weights_bin() {
        assert_eq!(
            get_bootstrap_weights_bin(Networks::NanoLiveNetwork).len(),
            6256,
            "expected live weights don't match'"
        );
        assert_eq!(
            get_bootstrap_weights_bin(Networks::NanoBetaNetwork).len(),
            0,
            "expected beta weights don't match'"
        );
    }

    #[test]
    fn bootstrap_weights() {
        let (max_blocks, weights) = get_bootstrap_weights(Networks::NanoLiveNetwork);
        assert_eq!(weights.len(), 130);
        assert_eq!(max_blocks, 184_789_962);
    }

    #[test]
    fn start_peer_cache_updater() {
        let node = TestNode::new();
        let start_tracker = node.peer_cache_updater.track_start();

        node.start();

        assert_eq!(
            start_tracker.output(),
            vec![TimerStartEvent {
                thread_name: "Peer history".to_string(),
                interval: Duration::from_secs(15),
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
            let config = NodeConfig::new_null();
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
                Box::new(|_, _, _| {}),
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