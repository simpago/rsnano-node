mod account_sets;
mod crawlers;
mod database_scan;
mod frontier_scan;
mod ordered_blocking;
mod ordered_heads;
mod ordered_priorities;
mod ordered_tags;
mod peer_scoring;
mod priority;
mod throttle;

use self::{
    account_sets::*,
    ordered_tags::{AsyncTag, OrderedTags},
    peer_scoring::PeerScoring,
    throttle::Throttle,
};
use crate::{
    block_processing::{BlockProcessor, BlockProcessorContext, BlockSource},
    bootstrap::{ascending::ordered_tags::QueryType, BootstrapServer},
    stats::{DetailType, Direction, Sample, StatType, Stats},
    transport::MessagePublisher,
    utils::{ThreadPool, ThreadPoolImpl},
};
pub use account_sets::AccountSetsConfig;
use crawlers::{AccountDatabaseCrawler, PendingDatabaseCrawler};
use database_scan::DatabaseScan;
use frontier_scan::{FrontierScan, FrontierScanConfig};
use num::clamp;
use ordered_tags::QuerySource;
use priority::Priority;
use rand::{thread_rng, RngCore};
use rsnano_core::{
    utils::ContainerInfo, Account, AccountInfo, Block, BlockHash, BlockType, Frontier,
    HashOrAccount, SavedBlock,
};
use rsnano_ledger::{BlockStatus, Ledger};
use rsnano_messages::{
    AccountInfoAckPayload, AccountInfoReqPayload, AscPullAck, AscPullAckType, AscPullReq,
    AscPullReqType, BlocksAckPayload, BlocksReqPayload, FrontiersReqPayload, HashType, Message,
};
use rsnano_network::{
    bandwidth_limiter::RateLimiter, ChannelId, DropPolicy, NetworkInfo, TrafficType,
};
use rsnano_nullable_clock::{SteadyClock, Timestamp};
use std::{
    cmp::{max, min},
    sync::{Arc, Condvar, Mutex, RwLock},
    thread::JoinHandle,
    time::{Duration, Instant},
};
use tracing::warn;

enum VerifyResult {
    Ok,
    NothingNew,
    Invalid,
}

pub struct BootstrapAscending {
    block_processor: Arc<BlockProcessor>,
    ledger: Arc<Ledger>,
    stats: Arc<Stats>,
    message_publisher: Mutex<MessagePublisher>,
    threads: Mutex<Option<Threads>>,
    mutex: Arc<Mutex<BootstrapAscendingLogic>>,
    condition: Arc<Condvar>,
    config: BootstrapAscendingConfig,
    /// Requests for accounts from database have much lower hitrate and could introduce strain on the network
    /// A separate (lower) limiter ensures that we always reserve resources for querying accounts from priority queue
    database_limiter: RateLimiter,
    frontiers_limiter: RateLimiter,
    clock: Arc<SteadyClock>,
    workers: ThreadPoolImpl,
}

struct Threads {
    timeout: JoinHandle<()>,
    priorities: Option<JoinHandle<()>>,
    database: Option<JoinHandle<()>>,
    dependencies: Option<JoinHandle<()>>,
    frontiers: Option<JoinHandle<()>>,
}

impl BootstrapAscending {
    pub(crate) fn new(
        block_processor: Arc<BlockProcessor>,
        ledger: Arc<Ledger>,
        stats: Arc<Stats>,
        network_info: Arc<RwLock<NetworkInfo>>,
        message_publisher: MessagePublisher,
        config: BootstrapAscendingConfig,
        clock: Arc<SteadyClock>,
    ) -> Self {
        Self {
            block_processor,
            threads: Mutex::new(None),
            mutex: Arc::new(Mutex::new(BootstrapAscendingLogic {
                stopped: false,
                accounts: AccountSets::new(config.account_sets.clone()),
                scoring: PeerScoring::new(config.clone()),
                database_scan: DatabaseScan::new(ledger.clone()),
                frontiers: FrontierScan::new(
                    config.frontier_scan.clone(),
                    stats.clone(),
                    clock.clone(),
                ),
                tags: OrderedTags::default(),
                throttle: Throttle::new(compute_throttle_size(
                    ledger.account_count(),
                    config.throttle_coefficient,
                )),
                sync_dependencies_interval: Instant::now(),
                config: config.clone(),
                network_info,
            })),
            condition: Arc::new(Condvar::new()),
            database_limiter: RateLimiter::new(config.database_rate_limit),
            frontiers_limiter: RateLimiter::new(config.frontier_rate_limit),
            config,
            stats,
            ledger,
            message_publisher: Mutex::new(message_publisher),
            clock,
            workers: ThreadPoolImpl::create(1, "Ascboot work"),
        }
    }

    pub fn stop(&self) {
        self.mutex.lock().unwrap().stopped = true;
        self.condition.notify_all();
        let threads = self.threads.lock().unwrap().take();
        if let Some(threads) = threads {
            if let Some(handle) = threads.priorities {
                handle.join().unwrap();
            }
            threads.timeout.join().unwrap();
            if let Some(database) = threads.database {
                database.join().unwrap();
            }
            if let Some(dependencies) = threads.dependencies {
                dependencies.join().unwrap();
            }
            if let Some(frontiers) = threads.frontiers {
                frontiers.join().unwrap();
            }
        }
    }

    fn send(&self, channel_id: ChannelId, request: &Message) {
        self.stats
            .inc(StatType::BootstrapAscending, DetailType::Request);

        let query_type = QueryType::from(request);
        self.stats
            .inc(StatType::BootstrapAscendingRequest, query_type.into());

        // TODO: There is no feedback mechanism if bandwidth limiter starts dropping our requests
        self.message_publisher.lock().unwrap().try_send(
            channel_id,
            &request,
            DropPolicy::CanDrop,
            TrafficType::Bootstrap,
        );
    }

    fn create_asc_pull_request(&self, tag: &AsyncTag) -> Message {
        debug_assert!(tag.source != QuerySource::Invalid);

        {
            let mut guard = self.mutex.lock().unwrap();
            debug_assert!(!guard.tags.contains(tag.id));
            guard.tags.insert(tag.clone());
        }

        // TODO: on_request.notify(tag, channel);

        let req_type = match tag.query_type {
            QueryType::BlocksByHash | QueryType::BlocksByAccount => {
                let start_type = if tag.query_type == QueryType::BlocksByHash {
                    HashType::Block
                } else {
                    HashType::Account
                };

                AscPullReqType::Blocks(BlocksReqPayload {
                    start_type,
                    start: tag.start,
                    count: tag.count as u8,
                })
            }
            QueryType::AccountInfoByHash => AscPullReqType::AccountInfo(AccountInfoReqPayload {
                target: tag.start,
                target_type: HashType::Block, // Query account info by block hash
            }),
            QueryType::Invalid => panic!("invalid query type"),
            QueryType::Frontiers => {
                AscPullReqType::Frontiers(rsnano_messages::FrontiersReqPayload {
                    start: tag.start.into(),
                    count: FrontiersReqPayload::MAX_FRONTIERS,
                })
            }
        };

        Message::AscPullReq(AscPullReq {
            id: tag.id,
            req_type,
        })
    }

    pub fn priority_len(&self) -> usize {
        self.mutex.lock().unwrap().accounts.priority_len()
    }

    pub fn blocked_len(&self) -> usize {
        self.mutex.lock().unwrap().accounts.blocked_len()
    }

    pub fn score_len(&self) -> usize {
        self.mutex.lock().unwrap().scoring.len()
    }

    pub fn prioritized(&self, account: &Account) -> bool {
        self.mutex.lock().unwrap().accounts.prioritized(account)
    }

    pub fn blocked(&self, account: &Account) -> bool {
        self.mutex.lock().unwrap().accounts.blocked(account)
    }

    /* Waits for a condition to be satisfied with incremental backoff */
    fn wait(&self, mut predicate: impl FnMut(&mut BootstrapAscendingLogic) -> bool) {
        let mut guard = self.mutex.lock().unwrap();
        let mut interval = Duration::from_millis(5);
        while !guard.stopped && !predicate(&mut guard) {
            guard = self
                .condition
                .wait_timeout_while(guard, interval, |g| !g.stopped)
                .unwrap()
                .0;
            interval = min(interval * 2, self.config.throttle_wait);
        }
    }

    /* Avoid too many in-flight requests */
    fn wait_tags(&self) {
        self.wait(|i| i.tags.len() < self.config.max_requests);
    }

    /* Ensure there is enough space in blockprocessor for queuing new blocks */
    fn wait_blockprocessor(&self) {
        self.wait(|_| {
            self.block_processor.queue_len(BlockSource::Bootstrap)
                < self.config.block_processor_theshold
        });
    }

    /* Waits for a channel that is not full */
    fn wait_channel(&self) -> Option<ChannelId> {
        let mut channel_id: Option<ChannelId> = None;
        self.wait(|i| {
            channel_id = i.scoring.channel().map(|c| c.channel_id());
            channel_id.is_some() // Wait until a channel is available
        });

        channel_id
    }

    fn wait_priority(&self) -> (Account, Priority) {
        let mut result = (Account::zero(), Priority::ZERO);
        self.wait(|i| {
            result = i.next_priority(&self.stats, self.clock.now());
            !result.0.is_zero()
        });
        result
    }

    fn wait_database(&self, should_throttle: bool) -> Account {
        let mut result = Account::zero();
        self.wait(|i| {
            result = i.next_database(
                should_throttle,
                &self.database_limiter,
                &self.stats,
                self.config.database_warmup_ratio,
            );
            !result.is_zero()
        });

        result
    }

    fn wait_blocking(&self) -> BlockHash {
        let mut result = BlockHash::zero();
        self.wait(|i| {
            result = i.next_blocking(&self.stats);
            !result.is_zero()
        });
        result
    }

    fn request(&self, account: Account, count: usize, channel_id: ChannelId, source: QuerySource) {
        let account_info = {
            let tx = self.ledger.read_txn();
            self.ledger.store.account.get(&tx, &account)
        };
        let id = thread_rng().next_u64();
        let now = self.clock.now();

        let request = self.create_blocks_request(id, account, account_info, count, source, now);

        self.send(channel_id, &request);
    }

    fn create_blocks_request(
        &self,
        id: u64,
        account: Account,
        account_info: Option<AccountInfo>,
        count: usize,
        source: QuerySource,
        now: Timestamp,
    ) -> Message {
        // Limit the max number of blocks to pull
        debug_assert!(count > 0);
        debug_assert!(count <= BootstrapServer::MAX_BLOCKS);
        let count = min(count, self.config.max_pull_count);

        // Check if the account picked has blocks, if it does, start the pull from the highest block
        let (query_type, start, hash) = match account_info {
            Some(info) => (
                QueryType::BlocksByHash,
                HashOrAccount::from(info.head),
                info.head,
            ),
            None => (
                QueryType::BlocksByAccount,
                HashOrAccount::from(account),
                BlockHash::zero(),
            ),
        };

        let tag = AsyncTag {
            id,
            account,
            timestamp: now,
            query_type,
            start,
            source,
            hash,
            count,
        };

        self.create_asc_pull_request(&tag)
    }

    fn create_account_info_request(
        &self,
        id: u64,
        hash: BlockHash,
        source: QuerySource,
        now: Timestamp,
    ) -> Message {
        let tag = AsyncTag {
            query_type: QueryType::AccountInfoByHash,
            source,
            start: hash.into(),
            account: Account::zero(),
            hash,
            count: 0,
            id,
            timestamp: now,
        };

        self.create_asc_pull_request(&tag)
    }

    fn run_one_priority(&self) {
        self.wait_tags();
        self.wait_blockprocessor();
        let Some(channel_id) = self.wait_channel() else {
            return;
        };

        let (account, priority) = self.wait_priority();
        if account.is_zero() {
            return;
        }

        let min_pull_count = 2;
        let count = clamp(
            f64::from(priority) as usize,
            min_pull_count,
            BootstrapServer::MAX_BLOCKS,
        );

        self.request(account, count, channel_id, QuerySource::Priority);
    }

    fn run_priorities(&self) {
        let mut guard = self.mutex.lock().unwrap();
        while !guard.stopped {
            drop(guard);
            self.stats
                .inc(StatType::BootstrapAscending, DetailType::Loop);
            self.run_one_priority();
            guard = self.mutex.lock().unwrap();
        }
    }

    fn run_one_database(&self, should_throttle: bool) {
        self.wait_tags();
        self.wait_blockprocessor();
        let Some(channel_id) = self.wait_channel() else {
            return;
        };
        let account = self.wait_database(should_throttle);
        if account.is_zero() {
            return;
        }
        self.request(account, 2, channel_id, QuerySource::Database);
    }

    fn run_database(&self) {
        let mut guard = self.mutex.lock().unwrap();
        while !guard.stopped {
            // Avoid high churn rate of database requests
            let should_throttle = !guard.database_scan.warmed_up() && guard.throttle.throttled();
            drop(guard);
            self.stats
                .inc(StatType::BootstrapAscending, DetailType::LoopDatabase);
            self.run_one_database(should_throttle);
            guard = self.mutex.lock().unwrap();
        }
    }

    fn run_one_blocking(&self) {
        self.wait_tags();
        self.wait_blockprocessor();
        let Some(channel_id) = self.wait_channel() else {
            return;
        };
        let blocking = self.wait_blocking();
        if blocking.is_zero() {
            return;
        }

        let now = self.clock.now();
        let id = thread_rng().next_u64();
        let request = self.create_account_info_request(id, blocking, QuerySource::Blocking, now);

        self.send(channel_id, &request);
    }

    fn run_frontiers(&self) {
        let mut guard = self.mutex.lock().unwrap();
        while !guard.stopped {
            drop(guard);

            self.stats
                .inc(StatType::BootstrapAscending, DetailType::LoopFrontiers);
            self.run_one_frontier();

            guard = self.mutex.lock().unwrap();
        }
    }

    fn run_one_frontier(&self) {
        self.wait(|i| !i.accounts.priority_half_full());
        self.wait(|_| self.frontiers_limiter.should_pass(1));
        self.wait(|_| self.workers.num_queued_tasks() < self.config.frontier_scan.max_pending);
        self.wait_tags();
        let Some(channel) = self.wait_channel() else {
            return;
        };
        let frontier = self.wait_frontier();
        if frontier.is_zero() {
            return;
        }
        self.request_frontiers(frontier, channel, QuerySource::Frontiers);
    }

    fn request_frontiers(&self, start: Account, channel: ChannelId, source: QuerySource) {
        let id = thread_rng().next_u64();
        let timestamp = self.clock.now();
        let tag = AsyncTag {
            query_type: QueryType::Frontiers,
            source,
            start: start.into(),
            account: Account::zero(),
            hash: BlockHash::zero(),
            count: 0,
            id,
            timestamp,
        };
        let message = self.create_asc_pull_request(&tag);
        self.send(channel, &message);
    }

    fn wait_frontier(&self) -> Account {
        let mut result = Account::zero();
        self.wait(|i| {
            result = i.frontiers.next();
            if !result.is_zero() {
                self.stats
                    .inc(StatType::BootstrapAscendingNext, DetailType::NextFrontier);
                true
            } else {
                false
            }
        });

        result
    }

    fn run_dependencies(&self) {
        let mut guard = self.mutex.lock().unwrap();
        while !guard.stopped {
            drop(guard);
            self.stats
                .inc(StatType::BootstrapAscending, DetailType::LoopDependencies);
            self.run_one_blocking();
            guard = self.mutex.lock().unwrap();
        }
    }

    fn run_timeouts(&self) {
        let mut guard = self.mutex.lock().unwrap();
        while !guard.stopped {
            self.stats
                .inc(StatType::BootstrapAscending, DetailType::LoopCleanup);

            guard.cleanup_and_sync(self.ledger.account_count(), &self.stats, self.clock.now());

            guard = self
                .condition
                .wait_timeout_while(guard, Duration::from_secs(1), |g| !g.stopped)
                .unwrap()
                .0;
        }
    }

    /// Process `asc_pull_ack` message coming from network
    pub fn process(&self, message: AscPullAck, channel_id: ChannelId) {
        let mut guard = self.mutex.lock().unwrap();

        // Only process messages that have a known tag
        let Some(tag) = guard.tags.remove(message.id) else {
            self.stats
                .inc(StatType::BootstrapAscending, DetailType::MissingTag);
            return;
        };

        self.stats
            .inc(StatType::BootstrapAscending, DetailType::Reply);

        let valid = match message.pull_type {
            AscPullAckType::Blocks(_) => matches!(
                tag.query_type,
                QueryType::BlocksByHash | QueryType::BlocksByAccount
            ),
            AscPullAckType::AccountInfo(_) => tag.query_type == QueryType::AccountInfoByHash,
            AscPullAckType::Frontiers(_) => tag.query_type == QueryType::Frontiers,
        };

        if !valid {
            self.stats.inc(
                StatType::BootstrapAscending,
                DetailType::InvalidResponseType,
            );
            return;
        }

        // Track bootstrap request response time
        self.stats
            .inc(StatType::BootstrapAscendingReply, tag.query_type.into());

        self.stats.sample(
            Sample::BootstrapTagDuration,
            tag.timestamp.elapsed(self.clock.now()).as_millis() as i64,
            (0, self.config.request_timeout.as_millis() as i64),
        );

        guard.scoring.received_message(channel_id);
        drop(guard);

        // Process the response payload
        match message.pull_type {
            AscPullAckType::Blocks(blocks) => self.process_blocks(&blocks, &tag),
            AscPullAckType::AccountInfo(info) => self.process_accounts(&info, &tag),
            AscPullAckType::Frontiers(frontiers) => self.process_frontiers(frontiers, &tag),
        }

        self.condition.notify_all();
    }

    fn process_frontiers(&self, frontiers: Vec<Frontier>, tag: &AsyncTag) {
        debug_assert_eq!(tag.query_type, QueryType::Frontiers);
        debug_assert!(!tag.start.is_zero());

        if frontiers.is_empty() {
            self.stats.inc(
                StatType::BootstrapAscendingProcess,
                DetailType::FrontiersEmpty,
            );
            return;
        }

        self.stats
            .inc(StatType::BootstrapAscendingProcess, DetailType::Frontiers);

        let result = self.verify_frontiers(&frontiers, tag);
        match result {
            VerifyResult::Ok => {
                self.stats
                    .inc(StatType::BootstrapAscendingVerifyFrontiers, DetailType::Ok);
                self.stats.add_dir(
                    StatType::BootstrapAscending,
                    DetailType::Frontiers,
                    Direction::In,
                    frontiers.len() as u64,
                );

                {
                    let mut guard = self.mutex.lock().unwrap();
                    guard.frontiers.process(tag.start.into(), &frontiers);
                }

                // Allow some overfill to avoid unnecessarily dropping responses
                if self.workers.num_queued_tasks() < self.config.frontier_scan.max_pending * 4 {
                    let stats = self.stats.clone();
                    let ledger = self.ledger.clone();
                    let mutex = self.mutex.clone();
                    self.workers.post(Box::new(move || {
                        process_frontiers(ledger, stats, frontiers, mutex)
                    }));
                } else {
                    self.stats
                        .inc(StatType::BootstrapAscending, DetailType::DroppedFrontiers);
                }
            }
            VerifyResult::NothingNew => self.stats.inc(
                StatType::BootstrapAscendingVerifyFrontiers,
                DetailType::NothingNew,
            ),
            VerifyResult::Invalid => self.stats.inc(
                StatType::BootstrapAscendingVerifyFrontiers,
                DetailType::Invalid,
            ),
        }
    }

    fn verify_frontiers(&self, frontiers: &[Frontier], tag: &AsyncTag) -> VerifyResult {
        if frontiers.is_empty() {
            return VerifyResult::NothingNew;
        }

        // Ensure frontiers accounts are in ascending order
        let mut previous = Account::zero();
        for f in frontiers {
            if f.account.number() <= previous.number() {
                return VerifyResult::Invalid;
            }
            previous = f.account;
        }

        // Ensure the frontiers are larger or equal to the requested frontier
        if frontiers[0].account.number() < tag.start.number() {
            return VerifyResult::Invalid;
        }

        VerifyResult::Ok
    }

    fn process_blocks(&self, response: &BlocksAckPayload, tag: &AsyncTag) {
        self.stats
            .inc(StatType::BootstrapAscendingProcess, DetailType::Blocks);

        let result = verify_response(response, tag);
        match result {
            VerifyResult::Ok => {
                self.stats
                    .inc(StatType::BootstrapAscendingVerifyBlocks, DetailType::Ok);
                self.stats.add_dir(
                    StatType::BootstrapAscending,
                    DetailType::Blocks,
                    Direction::In,
                    response.blocks().len() as u64,
                );

                let mut blocks = response.blocks().clone();

                // Avoid re-processing the block we already have
                assert!(blocks.len() >= 1);
                if blocks.front().unwrap().hash() == tag.start.into() {
                    blocks.pop_front();
                }

                while let Some(block) = blocks.pop_front() {
                    if blocks.is_empty() {
                        // It's the last block submitted for this account chain, reset timestamp to allow more requests
                        let stats = self.stats.clone();
                        let data = self.mutex.clone();
                        let condition = self.condition.clone();
                        let account = tag.account;
                        self.block_processor.add_with_callback(
                            block,
                            BlockSource::Bootstrap,
                            ChannelId::LOOPBACK,
                            Box::new(move |_| {
                                stats.inc(StatType::BootstrapAscending, DetailType::TimestampReset);
                                {
                                    let mut guard = data.lock().unwrap();
                                    guard.accounts.timestamp_reset(&account);
                                }
                                condition.notify_all();
                            }),
                        );
                    } else {
                        self.block_processor.add(
                            block,
                            BlockSource::Bootstrap,
                            ChannelId::LOOPBACK,
                        );
                    }
                }

                if tag.source == QuerySource::Database {
                    self.mutex.lock().unwrap().throttle.add(true);
                }
            }
            VerifyResult::NothingNew => {
                self.stats.inc(
                    StatType::BootstrapAscendingVerifyBlocks,
                    DetailType::NothingNew,
                );

                let mut guard = self.mutex.lock().unwrap();
                match guard.accounts.priority_down(&tag.account) {
                    PriorityDownResult::Deprioritized => {
                        self.stats.inc(
                            StatType::BootstrapAscendingAccounts,
                            DetailType::Deprioritize,
                        );
                    }
                    PriorityDownResult::Erased => {
                        self.stats.inc(
                            StatType::BootstrapAscendingAccounts,
                            DetailType::Deprioritize,
                        );
                        self.stats.inc(
                            StatType::BootstrapAscendingAccounts,
                            DetailType::PriorityEraseThreshold,
                        );
                    }
                    PriorityDownResult::AccountNotFound => {
                        self.stats.inc(
                            StatType::BootstrapAscendingAccounts,
                            DetailType::DeprioritizeFailed,
                        );
                    }
                    PriorityDownResult::InvalidAccount => {}
                }
                if tag.source == QuerySource::Database {
                    guard.throttle.add(false);
                }
            }
            VerifyResult::Invalid => {
                self.stats.inc(
                    StatType::BootstrapAscendingVerifyBlocks,
                    DetailType::Invalid,
                );
            }
        }
    }

    fn process_accounts(&self, response: &AccountInfoAckPayload, tag: &AsyncTag) {
        if response.account.is_zero() {
            self.stats.inc(
                StatType::BootstrapAscendingProcess,
                DetailType::AccountInfoEmpty,
            );
            return;
        }

        self.stats
            .inc(StatType::BootstrapAscendingProcess, DetailType::AccountInfo);

        // Prioritize account containing the dependency
        {
            let mut guard = self.mutex.lock().unwrap();
            let updated = guard
                .accounts
                .dependency_update(&tag.hash, response.account);
            if updated > 0 {
                self.stats.add(
                    StatType::BootstrapAscendingAccounts,
                    DetailType::DependencyUpdate,
                    updated as u64,
                );
            } else {
                self.stats.inc(
                    StatType::BootstrapAscendingAccounts,
                    DetailType::DependencyUpdateFailed,
                );
            }

            if guard.accounts.priority_set(&response.account) {
                self.priority_inserted();
            } else {
                self.priority_insertion_failed()
            };
        }
    }

    fn priority_inserted(&self) {
        self.stats.inc(
            StatType::BootstrapAscendingAccounts,
            DetailType::PriorityInsert,
        );
    }

    fn priority_insertion_failed(&self) {
        self.stats.inc(
            StatType::BootstrapAscendingAccounts,
            DetailType::PrioritizeFailed,
        );
    }

    fn batch_processed(&self, batch: &[(BlockStatus, Arc<BlockProcessorContext>)]) {
        {
            let mut guard = self.mutex.lock().unwrap();
            let tx = self.ledger.read_txn();
            for (result, context) in batch {
                let block = context.block.lock().unwrap().clone();
                let saved_block = context.saved_block.lock().unwrap().clone();
                let account = block.account_field().unwrap_or_else(|| {
                    self.ledger
                        .any()
                        .block_account(&tx, &block.previous())
                        .unwrap_or_default()
                });

                guard.inspect(
                    &self.stats,
                    *result,
                    &block,
                    saved_block,
                    context.source,
                    &account,
                );
            }
        }

        self.condition.notify_all();
    }

    pub fn container_info(&self) -> ContainerInfo {
        self.mutex.lock().unwrap().container_info()
    }
}

impl Drop for BootstrapAscending {
    fn drop(&mut self) {
        // All threads must be stopped before destruction
        debug_assert!(self.threads.lock().unwrap().is_none());
    }
}

pub trait BootstrapAscendingExt {
    fn initialize(&self, genesis_account: &Account);
    fn start(&self);
}

impl BootstrapAscendingExt for Arc<BootstrapAscending> {
    fn initialize(&self, genesis_account: &Account) {
        let self_w = Arc::downgrade(self);
        self.block_processor
            .on_batch_processed(Box::new(move |batch| {
                if let Some(self_l) = self_w.upgrade() {
                    self_l.batch_processed(batch);
                }
            }));

        let inserted = self
            .mutex
            .lock()
            .unwrap()
            .accounts
            .priority_set(genesis_account);

        if inserted {
            self.priority_inserted()
        } else {
            self.priority_insertion_failed()
        };
    }

    fn start(&self) {
        debug_assert!(self.threads.lock().unwrap().is_none());

        if !self.config.enable {
            warn!("Ascending bootstrap is disabled");
            return;
        }

        let priorities = if self.config.enable_scan {
            let self_l = Arc::clone(self);
            Some(
                std::thread::Builder::new()
                    .name("Ascboot".to_string())
                    .spawn(Box::new(move || self_l.run_priorities()))
                    .unwrap(),
            )
        } else {
            None
        };

        let database = if self.config.enable_database_scan {
            let self_l = Arc::clone(self);
            Some(
                std::thread::Builder::new()
                    .name("Ascboot".to_string())
                    .spawn(Box::new(move || self_l.run_database()))
                    .unwrap(),
            )
        } else {
            None
        };

        let dependencies = if self.config.enable_dependency_walker {
            let self_l = Arc::clone(self);
            Some(
                std::thread::Builder::new()
                    .name("Ascboot".to_string())
                    .spawn(Box::new(move || self_l.run_dependencies()))
                    .unwrap(),
            )
        } else {
            None
        };

        let frontiers = if self.config.enable_frontier_scan {
            let self_l = Arc::clone(self);
            Some(
                std::thread::Builder::new()
                    .name("Ascboot".to_string())
                    .spawn(Box::new(move || self_l.run_frontiers()))
                    .unwrap(),
            )
        } else {
            None
        };

        let self_l = Arc::clone(self);
        let timeout = std::thread::Builder::new()
            .name("Ascboot".to_string())
            .spawn(Box::new(move || self_l.run_timeouts()))
            .unwrap();

        *self.threads.lock().unwrap() = Some(Threads {
            timeout,
            priorities,
            database,
            frontiers,
            dependencies,
        });
    }
}

struct BootstrapAscendingLogic {
    stopped: bool,
    accounts: AccountSets,
    scoring: PeerScoring,
    database_scan: DatabaseScan,
    tags: OrderedTags,
    throttle: Throttle,
    frontiers: FrontierScan,
    sync_dependencies_interval: Instant,
    config: BootstrapAscendingConfig,
    network_info: Arc<RwLock<NetworkInfo>>,
}

impl BootstrapAscendingLogic {
    /// Inspects a block that has been processed by the block processor
    /// - Marks an account as blocked if the result code is gap source as there is no reason request additional blocks for this account until the dependency is resolved
    /// - Marks an account as forwarded if it has been recently referenced by a block that has been inserted.
    fn inspect(
        &mut self,
        stats: &Stats,
        status: BlockStatus,
        block: &Block,
        saved_block: Option<SavedBlock>,
        source: BlockSource,
        account: &Account,
    ) {
        let hash = block.hash();

        match status {
            BlockStatus::Progress => {
                let saved_block = saved_block.unwrap();
                let account = saved_block.account();
                // If we've inserted any block in to an account, unmark it as blocked
                if self.accounts.unblock(account, None) {
                    stats.inc(StatType::BootstrapAscendingAccounts, DetailType::Unblock);
                } else {
                    stats.inc(
                        StatType::BootstrapAscendingAccounts,
                        DetailType::UnblockFailed,
                    );
                }

                match self.accounts.priority_up(&account) {
                    PriorityUpResult::Updated => {
                        stats.inc(StatType::BootstrapAscendingAccounts, DetailType::Prioritize);
                    }
                    PriorityUpResult::Inserted => {
                        stats.inc(StatType::BootstrapAscendingAccounts, DetailType::Prioritize);
                        stats.inc(
                            StatType::BootstrapAscendingAccounts,
                            DetailType::PriorityInsert,
                        );
                    }
                    PriorityUpResult::AccountBlocked => {
                        stats.inc(
                            StatType::BootstrapAscendingAccounts,
                            DetailType::PrioritizeFailed,
                        );
                    }
                    PriorityUpResult::InvalidAccount => {}
                }

                if saved_block.is_send() {
                    let destination = saved_block.destination().unwrap();
                    // Unblocking automatically inserts account into priority set
                    if self.accounts.unblock(destination, Some(hash)) {
                        stats.inc(StatType::BootstrapAscendingAccounts, DetailType::Unblock);
                    } else {
                        stats.inc(
                            StatType::BootstrapAscendingAccounts,
                            DetailType::UnblockFailed,
                        );
                    }
                    if self.accounts.priority_set(&destination) {
                        stats.inc(
                            StatType::BootstrapAscendingAccounts,
                            DetailType::PriorityInsert,
                        );
                    } else {
                        stats.inc(
                            StatType::BootstrapAscendingAccounts,
                            DetailType::PrioritizeFailed,
                        );
                    };
                }
            }
            BlockStatus::GapSource => {
                if source == BlockSource::Bootstrap {
                    let source = block.source_or_link();

                    if !account.is_zero() && !source.is_zero() {
                        // Mark account as blocked because it is missing the source block
                        self.accounts.block(*account, source);
                        stats.inc(StatType::BootstrapAscendingAccounts, DetailType::Block);
                        stats.inc(
                            StatType::BootstrapAscendingAccounts,
                            DetailType::PriorityEraseBlock,
                        );
                        stats.inc(
                            StatType::BootstrapAscendingAccounts,
                            DetailType::BlockingInsert,
                        );
                    }
                }
            }
            BlockStatus::GapPrevious => {
                // Prevent live traffic from evicting accounts from the priority list
                if source == BlockSource::Live
                    && !self.accounts.priority_half_full()
                    && !self.accounts.blocked_half_full()
                {
                    if block.block_type() == BlockType::State {
                        let account = block.account_field().unwrap();
                        if self.accounts.priority_set(&account) {
                            stats.inc(
                                StatType::BootstrapAscendingAccounts,
                                DetailType::PriorityInsert,
                            );
                        } else {
                            stats.inc(
                                StatType::BootstrapAscendingAccounts,
                                DetailType::PrioritizeFailed,
                            );
                        }
                    }
                }
            }
            _ => {
                // No need to handle other cases
            }
        }
    }

    fn count_tags_by_hash(&self, hash: &BlockHash, source: QuerySource) -> usize {
        self.tags
            .iter_hash(hash)
            .filter(|i| i.source == source)
            .count()
    }

    fn next_priority(&mut self, stats: &Stats, now: Timestamp) -> (Account, Priority) {
        let account = self.accounts.next_priority(now, |account| {
            self.tags.count_by_account(account, QuerySource::Priority) < 4
        });

        if account.is_zero() {
            return Default::default();
        }

        stats.inc(StatType::BootstrapAscendingNext, DetailType::NextPriority);
        self.accounts.timestamp_set(&account, now);

        // TODO: Priority could be returned by the accounts.next_priority() call
        (account, self.accounts.priority(&account))
    }

    /* Gets the next account from the database */
    fn next_database(
        &mut self,
        should_throttle: bool,
        database_limiter: &RateLimiter,
        stats: &Stats,
        warmup_ratio: usize,
    ) -> Account {
        debug_assert!(warmup_ratio > 0);

        // Throttling increases the weight of database requests
        if !database_limiter.should_pass(if should_throttle { warmup_ratio } else { 1 }) {
            return Account::zero();
        }

        let account = self
            .database_scan
            .next(|account| self.tags.count_by_account(account, QuerySource::Database) == 0);

        if account.is_zero() {
            return account;
        }

        stats.inc(StatType::BootstrapAscendingNext, DetailType::NextDatabase);

        account
    }

    /* Waits for next available blocking block */
    fn next_blocking(&self, stats: &Stats) -> BlockHash {
        let blocking = self
            .accounts
            .next_blocking(|hash| self.count_tags_by_hash(hash, QuerySource::Blocking) == 0);

        if blocking.is_zero() {
            return blocking;
        }

        stats.inc(StatType::BootstrapAscendingNext, DetailType::NextBlocking);

        blocking
    }

    fn cleanup_and_sync(&mut self, account_count: u64, stats: &Stats, now: Timestamp) {
        let channels = self.network_info.read().unwrap().list_realtime_channels(0);
        self.scoring.sync(&channels);
        self.scoring.timeout();

        self.throttle.resize(compute_throttle_size(
            account_count,
            self.config.throttle_coefficient,
        ));

        let cutoff = now - self.config.request_timeout;
        let should_timeout = |tag: &AsyncTag| tag.timestamp < cutoff;

        while let Some(front) = self.tags.front() {
            if !should_timeout(front) {
                break;
            }

            self.tags.pop_front();
            stats.inc(StatType::BootstrapAscending, DetailType::Timeout);
        }

        if self.sync_dependencies_interval.elapsed() >= Duration::from_secs(60) {
            self.sync_dependencies_interval = Instant::now();
            stats.inc(StatType::BootstrapAscending, DetailType::SyncDependencies);
            let (inserted, insert_failed) = self.accounts.sync_dependencies();
            stats.add(
                StatType::BootstrapAscendingAccounts,
                DetailType::PriorityInsert,
                inserted as u64,
            );
            stats.add(
                StatType::BootstrapAscendingAccounts,
                DetailType::PrioritizeFailed,
                insert_failed as u64,
            );
        }
    }

    pub fn container_info(&self) -> ContainerInfo {
        ContainerInfo::builder()
            .leaf("tags", self.tags.len(), OrderedTags::ELEMENT_SIZE)
            .leaf("throttle", self.throttle.len(), 0)
            .leaf("throttle_success", self.throttle.successes(), 0)
            .node("accounts", self.accounts.container_info())
            .node("database_scan", self.database_scan.container_info())
            .node("frontiers", self.frontiers.container_info())
            .finish()
    }
}

// Calculates a lookback size based on the size of the ledger where larger ledgers have a larger sample count
fn compute_throttle_size(account_count: u64, throttle_coefficient: usize) -> usize {
    let target = if account_count > 0 {
        throttle_coefficient * ((account_count as f64).ln() as usize)
    } else {
        0
    };
    const MIN_SIZE: usize = 16;
    max(target, MIN_SIZE)
}

/// Verifies whether the received response is valid. Returns:
/// - invalid: when received blocks do not correspond to requested hash/account or they do not make a valid chain
/// - nothing_new: when received response indicates that the account chain does not have more blocks
/// - ok: otherwise, if all checks pass
fn verify_response(response: &BlocksAckPayload, tag: &AsyncTag) -> VerifyResult {
    let blocks = response.blocks();
    if blocks.is_empty() {
        return VerifyResult::NothingNew;
    }
    if blocks.len() == 1 && blocks.front().unwrap().hash() == tag.start.into() {
        return VerifyResult::NothingNew;
    }
    if blocks.len() > tag.count {
        return VerifyResult::Invalid;
    }

    let first = blocks.front().unwrap();
    match tag.query_type {
        QueryType::BlocksByHash => {
            if first.hash() != tag.start.into() {
                // TODO: Stat & log
                return VerifyResult::Invalid;
            }
        }
        QueryType::BlocksByAccount => {
            // Open & state blocks always contain account field
            if first.account_field().unwrap() != tag.start.into() {
                // TODO: Stat & log
                return VerifyResult::Invalid;
            }
        }
        QueryType::AccountInfoByHash | QueryType::Frontiers | QueryType::Invalid => {
            return VerifyResult::Invalid;
        }
    }

    // Verify blocks make a valid chain
    let mut previous_hash = first.hash();
    for block in blocks.iter().skip(1) {
        if block.previous() != previous_hash {
            // TODO: Stat & log
            return VerifyResult::Invalid; // Blocks do not make a chain
        }
        previous_hash = block.hash();
    }

    VerifyResult::Ok
}

#[derive(Clone, Debug, PartialEq)]
pub struct BootstrapAscendingConfig {
    pub enable: bool,
    pub enable_scan: bool,
    pub enable_database_scan: bool,
    pub enable_dependency_walker: bool,
    pub enable_frontier_scan: bool,
    /// Maximum number of un-responded requests per channel, should be lower or equal to bootstrap server max queue size
    pub channel_limit: usize,
    pub database_rate_limit: usize,
    pub frontier_rate_limit: usize,
    pub database_warmup_ratio: usize,
    pub max_pull_count: usize,
    pub request_timeout: Duration,
    pub throttle_coefficient: usize,
    pub throttle_wait: Duration,
    pub block_processor_theshold: usize,
    /** Minimum accepted protocol version used when bootstrapping */
    pub min_protocol_version: u8,
    pub max_requests: usize,
    pub account_sets: AccountSetsConfig,
    pub frontier_scan: FrontierScanConfig,
}

impl Default for BootstrapAscendingConfig {
    fn default() -> Self {
        Self {
            enable: true,
            enable_scan: true,
            enable_database_scan: false,
            enable_dependency_walker: true,
            enable_frontier_scan: true,
            channel_limit: 16,
            database_rate_limit: 256,
            frontier_rate_limit: 8,
            database_warmup_ratio: 10,
            max_pull_count: BlocksAckPayload::MAX_BLOCKS,
            request_timeout: Duration::from_secs(3),
            throttle_coefficient: 8 * 1024,
            throttle_wait: Duration::from_millis(100),
            block_processor_theshold: 1000,
            min_protocol_version: 0x14, // TODO don't hard code
            max_requests: 1024,
            account_sets: Default::default(),
            frontier_scan: Default::default(),
        }
    }
}

impl From<&Message> for QueryType {
    fn from(value: &Message) -> Self {
        if let Message::AscPullReq(req) = value {
            match &req.req_type {
                AscPullReqType::Blocks(b) => match b.start_type {
                    HashType::Account => QueryType::BlocksByAccount,
                    HashType::Block => QueryType::BlocksByHash,
                },
                AscPullReqType::AccountInfo(_) => QueryType::AccountInfoByHash,
                AscPullReqType::Frontiers(_) => QueryType::Frontiers,
            }
        } else {
            QueryType::Invalid
        }
    }
}

fn process_frontiers(
    ledger: Arc<Ledger>,
    stats: Arc<Stats>,
    frontiers: Vec<Frontier>,
    mutex: Arc<Mutex<BootstrapAscendingLogic>>,
) {
    assert!(!frontiers.is_empty());

    stats.inc(StatType::BootstrapAscending, DetailType::ProcessFrontiers);
    let mut outdated = 0;
    let mut pending = 0;

    // Accounts with outdated frontiers to sync
    let mut result = Vec::new();
    {
        let tx = ledger.read_txn();

        let start = frontiers[0].account;
        let mut account_crawler = AccountDatabaseCrawler::new(&ledger, &tx);
        let mut pending_crawler = PendingDatabaseCrawler::new(&ledger, &tx);
        account_crawler.initialize(start);
        pending_crawler.initialize(start);

        let mut should_prioritize = |frontier: &Frontier| {
            account_crawler.advance_to(&frontier.account);
            pending_crawler.advance_to(&frontier.account);

            // Check if account exists in our ledger
            if let Some((cur_acc, info)) = &account_crawler.current {
                if *cur_acc == frontier.account {
                    // Check for frontier mismatch
                    if info.head != frontier.hash {
                        // Check if frontier block exists in our ledger
                        if !ledger.any().block_exists_or_pruned(&tx, &frontier.hash) {
                            outdated += 1;
                            return true; // Frontier is outdated
                        }
                    }
                    return false; // Account exists and frontier is up-to-date
                }
            }

            // Check if account has pending blocks in our ledger
            if let Some((key, _)) = &pending_crawler.current {
                if key.receiving_account == frontier.account {
                    pending += 1;
                    return true; // Account doesn't exist but has pending blocks in the ledger
                }
            }

            false // Account doesn't exist in the ledger and has no pending blocks, can't be prioritized right now
        };

        for frontier in &frontiers {
            if should_prioritize(frontier) {
                result.push(frontier.account);
            }
        }
    }

    stats.add(
        StatType::BootstrapAscending,
        DetailType::FrontiersProcessed,
        frontiers.len() as u64,
    );
    stats.add(
        StatType::BootstrapAscending,
        DetailType::FrontiersPrioritized,
        result.len() as u64,
    );
    stats.add(
        StatType::BootstrapAscending,
        DetailType::FrontiersOutdated,
        outdated,
    );
    stats.add(
        StatType::BootstrapAscending,
        DetailType::FrontiersPending,
        pending,
    );

    let mut guard = mutex.lock().unwrap();
    for account in result {
        guard.accounts.priority_set(&account);
    }
}
