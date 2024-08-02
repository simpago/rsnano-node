mod account_sets;
mod iterator;
mod ordered_blocking;
mod ordered_priorities;
mod ordered_tags;
mod peer_scoring;
mod throttle;

use self::{
    account_sets::*,
    iterator::BufferedIterator,
    ordered_tags::{AsyncTag, OrderedTags},
    peer_scoring::PeerScoring,
    throttle::Throttle,
};
use crate::{
    block_processing::{BlockProcessor, BlockSource},
    bootstrap::ascending::ordered_tags::QueryType,
    stats::{DetailType, Direction, Sample, StatType, Stats},
    transport::{BandwidthLimiter, BufferDropPolicy, ChannelEnum, Network, TrafficType},
};
pub use account_sets::AccountSetsConfig;
use num::integer::sqrt;
use rand::{thread_rng, RngCore};
use rsnano_core::{
    utils::{ContainerInfo, ContainerInfoComponent},
    Account, BlockEnum, HashOrAccount,
};
use rsnano_ledger::{BlockStatus, Ledger};
use rsnano_messages::{
    AscPullAck, AscPullAckType, AscPullReq, AscPullReqType, BlocksAckPayload, BlocksReqPayload,
    HashType, Message,
};
use rsnano_store_lmdb::LmdbReadTransaction;
use std::{
    sync::{Arc, Condvar, Mutex, MutexGuard},
    thread::JoinHandle,
    time::{Duration, Instant},
};

enum VerifyResult {
    Ok,
    NothingNew,
    Invalid,
}

pub struct BootstrapAscending {
    block_processor: Arc<BlockProcessor>,
    ledger: Arc<Ledger>,
    stats: Arc<Stats>,
    network: Arc<Network>,
    thread: Mutex<Option<JoinHandle<()>>>,
    timeout_thread: Mutex<Option<JoinHandle<()>>>,
    mutex: Mutex<BootstrapAscendingImpl>,
    condition: Condvar,
    config: BootstrapAscendingConfig,
    /// Requests for accounts from database have much lower hitrate and could introduce strain on the network
    /// A separate (lower) limiter ensures that we always reserve resources for querying accounts from priority queue
    database_limiter: BandwidthLimiter,
}

impl BootstrapAscending {
    pub fn new(
        block_processor: Arc<BlockProcessor>,
        ledger: Arc<Ledger>,
        stats: Arc<Stats>,
        network: Arc<Network>,
        config: BootstrapAscendingConfig,
    ) -> Self {
        Self {
            block_processor,
            thread: Mutex::new(None),
            timeout_thread: Mutex::new(None),
            mutex: Mutex::new(BootstrapAscendingImpl {
                stopped: false,
                accounts: AccountSets::new(Arc::clone(&stats), config.account_sets.clone()),
                scoring: PeerScoring::new(config.clone()),
                iterator: BufferedIterator::new(Arc::clone(&ledger)),
                tags: OrderedTags::default(),
                throttle: Throttle::new(compute_throttle_size(&ledger, &config)),
            }),
            condition: Condvar::new(),
            database_limiter: BandwidthLimiter::new(1.0, config.database_requests_limit),
            config,
            stats,
            network,
            ledger,
        }
    }

    pub fn stop(&self) {
        self.mutex.lock().unwrap().stopped = true;
        self.condition.notify_all();
        if let Some(handle) = self.thread.lock().unwrap().take() {
            handle.join().unwrap();
        }
        if let Some(handle) = self.timeout_thread.lock().unwrap().take() {
            handle.join().unwrap();
        }
    }

    fn send(&self, channel: &Arc<ChannelEnum>, tag: AsyncTag) {
        debug_assert!(matches!(
            tag.query_type,
            QueryType::BlocksByHash | QueryType::BlocksByAccount
        ));

        let request_payload = BlocksReqPayload {
            start_type: if tag.query_type == QueryType::BlocksByHash {
                HashType::Block
            } else {
                HashType::Account
            },
            start: tag.start,
            count: self.config.pull_count as u8,
        };
        let request = Message::AscPullReq(AscPullReq {
            id: tag.id,
            req_type: AscPullReqType::Blocks(request_payload),
        });

        self.stats.inc_dir(
            StatType::BootstrapAscending,
            DetailType::Request,
            Direction::Out,
        );

        // TODO: There is no feedback mechanism if bandwidth limiter starts dropping our requests
        channel.try_send(&request, BufferDropPolicy::Limiter, TrafficType::Bootstrap);
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

    fn wait_blockprocessor(&self) {
        let mut guard = self.mutex.lock().unwrap();
        while !guard.stopped
            && self.block_processor.queue_len(BlockSource::Bootstrap) > self.config.block_wait_count
        {
            // Blockprocessor is relatively slow, sleeping here instead of using conditions
            guard = self
                .condition
                .wait_timeout_while(guard, self.config.throttle_wait, |g| !g.stopped)
                .unwrap()
                .0;
        }
    }

    fn wait_available_channel(&self) -> Option<Arc<ChannelEnum>> {
        let mut guard = self.mutex.lock().unwrap();
        while !guard.stopped {
            let channel = guard.scoring.channel();
            if channel.is_some() {
                return channel;
            }

            let sleep = self.config.throttle_wait;
            guard = self
                .condition
                .wait_timeout_while(guard, sleep, |g| !g.stopped)
                .unwrap()
                .0;
        }

        None
    }

    fn wait_available_account(&self) -> Account {
        let mut guard = self.mutex.lock().unwrap();
        while !guard.stopped {
            let account = guard.available_account(&self.stats, &self.database_limiter);
            if !account.is_zero() {
                guard.accounts.timestamp(&account, false);
                return account;
            } else {
                guard = self
                    .condition
                    .wait_timeout_while(guard, Duration::from_millis(100), |g| !g.stopped)
                    .unwrap()
                    .0
            }
        }

        Account::zero()
    }

    fn request(&self, account: Account, channel: &Arc<ChannelEnum>) -> bool {
        let info = {
            let tx = self.ledger.read_txn();
            self.ledger.store.account.get(&tx, &account)
        };

        // Check if the account picked has blocks, if it does, start the pull from the highest block
        let (query_type, start) = match info {
            Some(info) => (QueryType::BlocksByHash, HashOrAccount::from(info.head)),
            None => (QueryType::BlocksByAccount, HashOrAccount::from(account)),
        };

        let tag = AsyncTag {
            id: thread_rng().next_u64(),
            account,
            time: Instant::now(),
            query_type,
            start,
        };

        self.track(tag.clone());
        self.send(channel, tag);
        true // Request sent
    }

    fn run_one(&self) -> bool {
        // Ensure there is enough space in blockprocessor for queuing new blocks
        self.wait_blockprocessor();

        // Waits for account either from priority queue or database
        let account = self.wait_available_account();
        if account.is_zero() {
            return false;
        }

        // Waits for channel that is not full
        let Some(channel) = self.wait_available_channel() else {
            return false;
        };

        let success = self.request(account, &channel);
        return success;
    }

    fn throttle_if_needed<'a>(
        &'a self,
        data: MutexGuard<'a, BootstrapAscendingImpl>,
    ) -> MutexGuard<'a, BootstrapAscendingImpl> {
        if !data.iterator.warmup() && data.throttle.throttled() {
            self.stats
                .inc(StatType::BootstrapAscending, DetailType::Throttled);
            self.condition
                .wait_timeout_while(data, self.config.throttle_wait, |g| !g.stopped)
                .unwrap()
                .0
        } else {
            data
        }
    }

    fn run(&self) {
        let mut guard = self.mutex.lock().unwrap();
        while !guard.stopped {
            drop(guard);
            self.stats
                .inc(StatType::BootstrapAscending, DetailType::Loop);
            self.run_one();
            guard = self.mutex.lock().unwrap();
            guard = self.throttle_if_needed(guard);
        }
    }

    fn run_timeouts(&self) {
        let mut guard = self.mutex.lock().unwrap();
        while !guard.stopped {
            guard.scoring.sync(&self.network.list_channels(0));
            guard.scoring.timeout();
            guard
                .throttle
                .resize(compute_throttle_size(&self.ledger, &self.config));
            while let Some(front) = guard.tags.front() {
                if front.time.elapsed() <= self.config.timeout {
                    break;
                }

                guard.tags.pop_front();
                self.stats
                    .inc(StatType::BootstrapAscending, DetailType::Timeout);
            }
            guard = self
                .condition
                .wait_timeout_while(guard, Duration::from_secs(1), |g| !g.stopped)
                .unwrap()
                .0;
        }
    }

    /// Process `asc_pull_ack` message coming from network
    pub fn process(&self, message: &AscPullAck, channel: &Arc<ChannelEnum>) {
        let mut guard = self.mutex.lock().unwrap();

        // Only process messages that have a known tag
        if let Some(tag) = guard.tags.remove(message.id) {
            self.stats
                .inc(StatType::BootstrapAscending, DetailType::Reply);
            self.stats.sample(
                Sample::BootstrapTagDuration,
                tag.time.elapsed().as_millis() as i64,
                (0, self.config.timeout.as_millis() as i64),
            );

            guard.scoring.received_message(channel);
            drop(guard);

            self.condition.notify_all();

            match &message.pull_type {
                AscPullAckType::Blocks(blocks) => self.process_blocks(blocks, &tag),
                AscPullAckType::AccountInfo(_) => { /* TODO: Make use of account info */ }
                AscPullAckType::Frontiers(_) => { /* TODO: Make use of frontiers info */ }
            }
        } else {
            self.stats
                .inc(StatType::BootstrapAscending, DetailType::MissingTag);
        }
    }

    fn process_blocks(&self, response: &BlocksAckPayload, tag: &AsyncTag) {
        self.stats
            .inc(StatType::BootstrapAscending, DetailType::Process);

        let result = self.verify(response, tag);
        match result {
            VerifyResult::Ok => {
                self.stats.add_dir(
                    StatType::BootstrapAscending,
                    DetailType::Blocks,
                    Direction::In,
                    response.blocks().len() as u64,
                );

                for block in response.blocks() {
                    self.block_processor
                        .add(Arc::new(block.clone()), BlockSource::Bootstrap, None);
                }
                let mut guard = self.mutex.lock().unwrap();
                guard.throttle.add(true);
            }
            VerifyResult::NothingNew => {
                self.stats
                    .inc(StatType::BootstrapAscending, DetailType::NothingNew);

                let mut guard = self.mutex.lock().unwrap();
                guard.accounts.priority_down(&tag.account);
                guard.throttle.add(false);
            }
            VerifyResult::Invalid => {
                self.stats
                    .inc(StatType::BootstrapAscending, DetailType::Invalid);
                // TODO: Log
            }
        }
    }

    /// Verifies whether the received response is valid. Returns:
    /// - invalid: when received blocks do not correspond to requested hash/account or they do not make a valid chain
    /// - nothing_new: when received response indicates that the account chain does not have more blocks
    /// - ok: otherwise, if all checks pass
    fn verify(&self, response: &BlocksAckPayload, tag: &AsyncTag) -> VerifyResult {
        let blocks = response.blocks();
        if blocks.is_empty() {
            return VerifyResult::NothingNew;
        }
        if blocks.len() == 1 && blocks.first().unwrap().hash() == tag.start.into() {
            return VerifyResult::NothingNew;
        }

        let first = blocks.first().unwrap();
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
            QueryType::Invalid => {
                return VerifyResult::Invalid;
            }
        }

        // Verify blocks make a valid chain
        let mut previous_hash = first.hash();
        for block in &blocks[1..] {
            if block.previous() != previous_hash {
                // TODO: Stat & log
                return VerifyResult::Invalid; // Blocks do not make a chain
            }
            previous_hash = block.hash();
        }

        VerifyResult::Ok
    }

    fn track(&self, tag: AsyncTag) {
        self.stats
            .inc(StatType::BootstrapAscending, DetailType::Track);

        let mut guard = self.mutex.lock().unwrap();
        debug_assert!(!guard.tags.contains(tag.id));
        guard.tags.insert(tag);
    }

    pub fn collect_container_info(&self, name: impl Into<String>) -> ContainerInfoComponent {
        let guard = self.mutex.lock().unwrap();
        ContainerInfoComponent::Composite(
            name.into(),
            vec![
                ContainerInfoComponent::Leaf(ContainerInfo {
                    name: "tags".to_string(),
                    count: guard.tags.len(),
                    sizeof_element: OrderedTags::ELEMENT_SIZE,
                }),
                ContainerInfoComponent::Leaf(ContainerInfo {
                    name: "throttle".to_string(),
                    count: guard.throttle.len(),
                    sizeof_element: 0,
                }),
                ContainerInfoComponent::Leaf(ContainerInfo {
                    name: "throttle_success".to_string(),
                    count: guard.throttle.successes(),
                    sizeof_element: 0,
                }),
                guard.accounts.collect_container_info("accounts"),
            ],
        )
    }
}

impl Drop for BootstrapAscending {
    fn drop(&mut self) {
        // All threads must be stopped before destruction
        debug_assert!(self.thread.lock().unwrap().is_none());
        debug_assert!(self.timeout_thread.lock().unwrap().is_none());
    }
}

pub trait BootstrapAscendingExt {
    fn initialize(&self);
    fn start(&self);
}

impl BootstrapAscendingExt for Arc<BootstrapAscending> {
    fn initialize(&self) {
        let self_w = Arc::downgrade(self);
        self.block_processor
            .add_batch_processed_observer(Box::new(move |batch| {
                if let Some(self_l) = self_w.upgrade() {
                    let mut should_notify = false;
                    {
                        let mut guard = self_l.mutex.lock().unwrap();
                        let tx = self_l.ledger.read_txn();
                        for (result, context) in batch {
                            // Do not try to unnecessarily bootstrap live traffic chains
                            if context.source == BlockSource::Bootstrap {
                                guard.inspect(&self_l.ledger, &tx, *result, &context.block);
                                should_notify = true;
                            }
                        }
                    }

                    if should_notify {
                        self_l.condition.notify_all();
                    }
                }
            }))
    }

    fn start(&self) {
        debug_assert!(self.thread.lock().unwrap().is_none());
        debug_assert!(self.timeout_thread.lock().unwrap().is_none());

        let self_l = Arc::clone(self);
        *self.thread.lock().unwrap() = Some(
            std::thread::Builder::new()
                .name("Bootstrap asc".to_string())
                .spawn(Box::new(move || self_l.run()))
                .unwrap(),
        );

        let self_l = Arc::clone(self);
        *self.thread.lock().unwrap() = Some(
            std::thread::Builder::new()
                .name("Bootstrap asc".to_string())
                .spawn(Box::new(move || self_l.run_timeouts()))
                .unwrap(),
        );
    }
}

struct BootstrapAscendingImpl {
    stopped: bool,
    accounts: AccountSets,
    scoring: PeerScoring,
    iterator: BufferedIterator,
    tags: OrderedTags,
    throttle: Throttle,
}

impl BootstrapAscendingImpl {
    /// Inspects a block that has been processed by the block processor
    /// - Marks an account as blocked if the result code is gap source as there is no reason request additional blocks for this account until the dependency is resolved
    /// - Marks an account as forwarded if it has been recently referenced by a block that has been inserted.
    fn inspect(
        &mut self,
        ledger: &Ledger,
        tx: &LmdbReadTransaction,
        status: BlockStatus,
        block: &Arc<BlockEnum>,
    ) {
        let hash = block.hash();

        match status {
            BlockStatus::Progress => {
                let account = block.account();
                // If we've inserted any block in to an account, unmark it as blocked
                self.accounts.unblock(account, None);
                self.accounts.priority_up(&account);
                self.accounts
                    .timestamp(&account, /* reset timestamp */ true);

                if block.is_send() {
                    let destination = block.destination().unwrap();
                    self.accounts.unblock(destination, Some(hash)); // Unblocking automatically inserts account into priority set
                    self.accounts.priority_up(&destination);
                }
            }
            BlockStatus::GapSource => {
                let account = if block.previous().is_zero() {
                    block.account_field().unwrap()
                } else {
                    ledger.any().block_account(tx, &block.previous()).unwrap()
                };
                let source = block.source_or_link();

                // Mark account as blocked because it is missing the source block
                self.accounts.block(account, source);

                // TODO: Track stats
            }
            BlockStatus::Old | BlockStatus::GapPrevious => {
                // TODO: Track stats
            }
            _ => {
                // No need to handle other cases
            }
        }
    }

    fn available_account(&mut self, stats: &Stats, database_limiter: &BandwidthLimiter) -> Account {
        {
            let account = self.accounts.next();
            if !account.is_zero() {
                stats.inc(StatType::BootstrapAscending, DetailType::NextPriority);
                return account;
            }
        }

        if database_limiter.should_pass(1) {
            let account = self.iterator.next();
            if !account.is_zero() {
                stats.inc(StatType::BootstrapAscending, DetailType::NextDatabase);
                return account;
            }
        }

        stats.inc(StatType::BootstrapAscending, DetailType::NextNone);
        Account::zero()
    }
}

fn compute_throttle_size(ledger: &Ledger, config: &BootstrapAscendingConfig) -> usize {
    // Scales logarithmically with ledger block
    // Returns: config.throttle_coefficient * sqrt(block_count)
    let size_new = config.throttle_coefficient as u64 * sqrt(ledger.block_count());
    if size_new == 0 {
        16
    } else {
        size_new as usize
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BootstrapAscendingConfig {
    /// Maximum number of un-responded requests per channel
    pub requests_limit: usize,
    pub database_requests_limit: usize,
    pub pull_count: usize,
    pub timeout: Duration,
    pub throttle_coefficient: usize,
    pub throttle_wait: Duration,
    pub account_sets: AccountSetsConfig,
    pub block_wait_count: usize,
    /** Minimum accepted protocol version used when bootstrapping */
    pub min_protocol_version: u8,
}

impl Default for BootstrapAscendingConfig {
    fn default() -> Self {
        Self {
            requests_limit: 64,
            database_requests_limit: 1024,
            pull_count: BlocksAckPayload::MAX_BLOCKS,
            timeout: Duration::from_secs(3),
            throttle_coefficient: 16,
            throttle_wait: Duration::from_millis(100),
            account_sets: Default::default(),
            block_wait_count: 1000,
            min_protocol_version: 0x14,
        }
    }
}
