use super::{
    bootstrap_limits, BootstrapAttempt, BootstrapAttemptTrait, BootstrapCallbacks,
    BootstrapConnections, BootstrapConnectionsExt, BootstrapInitiator, BootstrapMode,
};
use crate::{
    block_processing::{BlockProcessor, BlockSource},
    bootstrap::PullInfo,
    config::NodeFlags,
    NetworkParams,
};
use anyhow::Result;
use rsnano_core::{
    utils::PropertyTree, Account, Amount, Block, BlockHash, BlockType, HashOrAccount,
};
use rsnano_ledger::Ledger;
use rsnano_network::ChannelId;
use rsnano_store_lmdb::Transaction;
use std::{
    cmp::max,
    collections::{hash_map::DefaultHasher, HashMap, HashSet, VecDeque},
    hash::{Hash, Hasher},
    sync::{atomic::Ordering, Arc, Mutex, MutexGuard, Weak},
    time::{Duration, Instant},
};
use tracing::debug;

struct LazyStateBacklogItem {
    link: HashOrAccount,
    balance: Amount,
    retry_limit: u32,
}

/**
 * Lazy bootstrap session. Started with a block hash, this will "trace down" the blocks obtained to find a connection to the ledger.
 * This attempts to quickly bootstrap a section of the ledger given a hash that's known to be confirmed.
 */
pub struct BootstrapAttemptLazy {
    attempt: BootstrapAttempt,
    flags: NodeFlags,
    connections: Arc<BootstrapConnections>,
    ledger: Arc<Ledger>,
    network_params: NetworkParams,
    block_processor: Arc<BlockProcessor>,
    data: Mutex<LazyData>,
}

struct LazyData {
    lazy_blocks: HashSet<u64>,
    lazy_start_time: Instant,
    lazy_blocks_count: usize,
    lazy_pulls: VecDeque<(HashOrAccount, u32)>,
    lazy_undefined_links: HashSet<BlockHash>,
    lazy_state_backlog: HashMap<BlockHash, LazyStateBacklogItem>,
    lazy_keys: HashSet<BlockHash>,
    lazy_balances: HashMap<BlockHash, Amount>,
    disable_legacy_bootstrap: bool,
    lazy_retry_limit: u32,
}

fn u64_hash(block_hash: &BlockHash) -> u64 {
    let mut hasher = DefaultHasher::new();
    block_hash.hash(&mut hasher);
    hasher.finish()
}

impl LazyData {
    fn lazy_blocks_insert(&mut self, hash: &BlockHash) {
        let inserted = self.lazy_blocks.insert(u64_hash(&hash));

        if inserted {
            self.lazy_blocks_count += 1;
        }
    }

    fn lazy_block_erase(&mut self, hash: &BlockHash) {
        let erased = self.lazy_blocks.remove(&u64_hash(hash));
        if erased {
            self.lazy_blocks_count -= 1;
        }
    }

    fn lazy_add(&mut self, hash_or_account: HashOrAccount, retry_limit: u32) {
        // Add only unknown blocks
        if !self.lazy_blocks_processed(&hash_or_account.into()) {
            self.lazy_pulls.push_back((hash_or_account, retry_limit));
        }
    }

    fn lazy_blocks_processed(&self, hash: &BlockHash) -> bool {
        self.lazy_blocks.contains(&u64_hash(hash))
    }

    fn lazy_backlog_cleanup(&mut self, attempt: &BootstrapAttempt, ledger: &Ledger) {
        let mut read_count = 0;
        let mut txn = ledger.read_txn();

        let lazy_state_backlog = &mut self.lazy_state_backlog;
        let lazy_pulls = &mut self.lazy_pulls;
        let lazy_blocks = &self.lazy_blocks;

        lazy_state_backlog.retain(|hash, next_block| {
            if attempt.stopped() {
                return true;
            }

            let mut lazy_add = |hash_or_account: HashOrAccount, retry_limit: u32| {
                // Add only unknown blocks
                if !lazy_blocks.contains(&u64_hash(&hash_or_account.into())) {
                    lazy_pulls.push_back((hash_or_account, retry_limit));
                }
            };

            let mut retain = true;
            if ledger.any().block_exists_or_pruned(&txn, hash) {
                if let Some(balance) = ledger.any().block_balance(&txn, hash) {
                    if balance <= next_block.balance {
                        lazy_add(next_block.link, next_block.retry_limit);
                    }
                } else {
                    // Not confirmed
                    lazy_add(next_block.link, self.lazy_retry_limit);
                }
                retain = false;
            } else {
                lazy_add((*hash).into(), next_block.retry_limit);
            }
            // We don't want to open read transactions for too long
            read_count += 1;
            if read_count & BootstrapAttemptLazy::BATCH_READ_SIZE == 0 {
                txn.refresh();
            }

            retain
        });
    }

    fn lazy_has_expired(&self) -> bool {
        let mut result = false;
        // Max 30 minutes run with enabled legacy bootstrap
        let max_lazy_time = if self.disable_legacy_bootstrap {
            Duration::from_secs(7 * 24 * 60 * 60)
        } else {
            Duration::from_secs(30 * 60)
        };
        if self.lazy_start_time.elapsed() >= max_lazy_time {
            result = true;
        } else if !self.disable_legacy_bootstrap
            && self.lazy_blocks_count > bootstrap_limits::LAZY_BLOCKS_RESTART_LIMIT
        {
            result = true;
        }
        result
    }

    fn lazy_finished(&mut self, attempt: &BootstrapAttempt, ledger: &Ledger) -> bool {
        if attempt.stopped() {
            return true;
        }

        let mut result = true;
        let mut read_count = 0;
        let mut txn = ledger.read_txn();

        while let Some(hash) = self.lazy_keys.iter().next().cloned() {
            if attempt.stopped() {
                break;
            }
            if ledger.any().block_exists_or_pruned(&txn, &hash) {
                self.lazy_keys.remove(&hash);
            } else {
                result = false;
                break;
            }
            // We don't want to open read transactions for too long
            read_count += 1;
            if read_count % BootstrapAttemptLazy::BATCH_READ_SIZE == 0 {
                txn.refresh();
            }
        }

        // Finish lazy bootstrap without lazy pulls (in combination with still_pulling ())
        if !result && self.lazy_pulls.is_empty() && self.lazy_state_backlog.is_empty() {
            result = true;
        }
        result
    }

    fn lazy_block_state_backlog_check(&mut self, block: &Block, hash: &BlockHash) {
        // Search unknown state blocks balances
        if let Some(next_block) = self.lazy_state_backlog.get(hash) {
            let link = next_block.link;
            // Retrieve balance for previous state & send blocks
            let balance = match &block {
                Block::State(i) => Some(i.balance()),
                Block::LegacySend(i) => Some(i.balance()),
                _ => None,
            };
            if let Some(balance) = balance {
                // balance
                if balance <= next_block.balance {
                    self.lazy_add(next_block.link, next_block.retry_limit); // link
                }
            }
            // Assumption for other legacy block types
            else if !self.lazy_undefined_links.contains(&next_block.link.into()) {
                self.lazy_add(link, self.lazy_retry_limit); // Head is not confirmed. It can be account or hash or non-existing
                self.lazy_undefined_links.insert(link.into());
            }
            self.lazy_state_backlog.remove(hash);
        }
    }
}

unsafe impl Send for BootstrapAttemptLazy {}
unsafe impl Sync for BootstrapAttemptLazy {}

impl BootstrapAttemptLazy {
    /// The maximum number of records to be read in while iterating over long lazy containers
    const BATCH_READ_SIZE: usize = 256;

    pub fn new(
        block_processor: Arc<BlockProcessor>,
        bootstrap_initiator: Weak<BootstrapInitiator>,
        ledger: Arc<Ledger>,
        id: String,
        incremental_id: u64,
        flags: NodeFlags,
        connections: Arc<BootstrapConnections>,
        network_params: NetworkParams,
        bootstrap_callbacks: BootstrapCallbacks,
    ) -> Result<Self> {
        Ok(Self {
            attempt: BootstrapAttempt::new(
                Arc::downgrade(&block_processor),
                bootstrap_initiator,
                Arc::clone(&ledger),
                id,
                BootstrapMode::Lazy,
                incremental_id,
                bootstrap_callbacks,
            )?,
            flags: flags.clone(),
            connections,
            ledger,
            network_params: network_params.clone(),
            block_processor,
            data: Mutex::new(LazyData {
                lazy_blocks: Default::default(),
                lazy_pulls: Default::default(),
                lazy_undefined_links: Default::default(),
                lazy_state_backlog: Default::default(),
                lazy_keys: Default::default(),
                lazy_balances: Default::default(),
                lazy_start_time: Instant::now(),
                lazy_blocks_count: 0,
                disable_legacy_bootstrap: flags.disable_legacy_bootstrap,
                lazy_retry_limit: network_params.bootstrap.lazy_retry_limit,
            }),
        })
    }

    fn process_block_lazy(
        &self,
        block: Block,
        _known_account: &Account,
        pull_blocks_processed: u64,
        max_blocks: u32,
        retry_limit: u32,
    ) -> bool {
        let mut stop_pull = false;
        let hash = block.hash();
        let lock = self.attempt.mutex.lock().unwrap();
        let mut data = self.data.lock().unwrap();
        // Processing new blocks
        if !data.lazy_blocks_processed(&hash) {
            // Search for new dependencies
            if block.source_field().is_some()
                && !self
                    .ledger
                    .any()
                    .block_exists_or_pruned(&self.ledger.read_txn(), &block.source_or_link())
                && block.source_or_link()
                    != BlockHash::from_bytes(*self.network_params.ledger.genesis_account.as_bytes())
            {
                data.lazy_add(block.source_or_link().into(), retry_limit);
            } else if block.block_type() == BlockType::State {
                self.lazy_block_state(&mut data, &block, retry_limit);
            }
            data.lazy_blocks_insert(&hash);
            // Adding lazy balances for first processed block in pull
            if pull_blocks_processed == 1 {
                let balance = match &block {
                    Block::State(i) => Some(i.balance()),
                    Block::LegacySend(i) => Some(i.balance()),
                    _ => None,
                };
                if let Some(balance) = balance {
                    data.lazy_balances.insert(hash, balance);
                }
            }
            // Clearing lazy balances for previous block
            if !block.previous().is_zero() && data.lazy_balances.contains_key(&block.previous()) {
                data.lazy_balances.remove(&block.previous());
            }
            data.lazy_block_state_backlog_check(&block, &hash);
            drop(lock);
            drop(data);
            self.block_processor
                .add(block, BlockSource::BootstrapLegacy, ChannelId::LOOPBACK);
        }
        // Force drop lazy bootstrap connection for long bulk_pull
        if pull_blocks_processed > max_blocks as u64 {
            stop_pull = true;
        }

        stop_pull
    }

    fn lazy_block_state(&self, data: &mut LazyData, block: &Block, retry_limit: u32) {
        let txn = self.ledger.read_txn();
        let balance = block.balance_field().unwrap();
        let link = block.link_field().unwrap();
        // If link is not epoch link or 0. And if block from link is unknown
        if !link.is_zero()
            && !self.ledger.is_epoch_link(&link)
            && !data.lazy_blocks_processed(&link.into())
            && !self.ledger.any().block_exists_or_pruned(&txn, &link.into())
        {
            let previous = block.previous();
            // If state block previous is 0 then source block required
            if previous.is_zero() {
                data.lazy_add(link.into(), retry_limit);
            }
            // In other cases previous block balance required to find out subtype of state block
            else if self.ledger.any().block_exists_or_pruned(&txn, &previous) {
                if let Some(previous_balance) = self.ledger.any().block_balance(&txn, &previous) {
                    if previous_balance <= balance {
                        data.lazy_add(link.into(), retry_limit);
                    }
                }
                // Else ignore pruned blocks
            }
            // Search balance of already processed previous blocks
            else if data.lazy_blocks_processed(&previous) {
                if let Some(previous_balance) = data.lazy_balances.get(&previous) {
                    if *previous_balance <= balance {
                        data.lazy_add(link.into(), retry_limit);
                    }
                    data.lazy_balances.remove(&previous);
                }
            }
            // Insert in backlog state blocks if previous wasn't already processed
            else {
                data.lazy_state_backlog.insert(
                    previous,
                    LazyStateBacklogItem {
                        link: link.into(),
                        balance,
                        retry_limit,
                    },
                );
            }
        }
    }

    fn lazy_pull_flush<'a>(
        &'a self,
        mut lock: MutexGuard<'a, u8>,
        mut data: MutexGuard<'a, LazyData>,
    ) -> (MutexGuard<'a, u8>, MutexGuard<'a, LazyData>) {
        const MAX_PULLS: u32 = bootstrap_limits::BOOTSTRAP_CONNECTION_SCALE_TARGET_BLOCKS * 3;
        if self.pulling() < MAX_PULLS {
            debug_assert!(self.network_params.bootstrap.lazy_max_pull_blocks <= u32::MAX);
            let batch_count = self.lazy_batch_size_locked(&data);
            let mut read_count = 0;
            let mut count = 0;
            let mut txn = self.ledger.read_txn();
            while !data.lazy_pulls.is_empty() && count < MAX_PULLS {
                let pull_start = data.lazy_pulls.pop_front().unwrap();
                // Recheck if block was already processed
                if !data.lazy_blocks_processed(&pull_start.0.into())
                    && !self
                        .ledger
                        .any()
                        .block_exists_or_pruned(&txn, &pull_start.0.into())
                {
                    drop(data);
                    drop(lock);
                    let pull = PullInfo {
                        account_or_head: pull_start.0,
                        head: pull_start.0.into(),
                        head_original: pull_start.0.into(),
                        end: BlockHash::zero(),
                        count: batch_count,
                        attempts: 0,
                        processed: 0,
                        retry_limit: pull_start.1,
                        bootstrap_id: self.attempt.incremental_id,
                    };
                    self.connections.add_pull(pull);
                    self.attempt.pulling.fetch_add(1, Ordering::SeqCst);
                    count += 1;
                    lock = self.attempt.mutex.lock().unwrap();
                    data = self.data.lock().unwrap();
                }
                // We don't want to open read transactions for too long
                read_count += 1;
                if read_count % Self::BATCH_READ_SIZE == 0 {
                    drop(data);
                    drop(lock);
                    txn.refresh();
                    lock = self.attempt.mutex.lock().unwrap();
                    data = self.data.lock().unwrap();
                }
            }
        }
        (lock, data)
    }

    pub fn lazy_processed_or_exists(&self, hash: &BlockHash) -> bool {
        let mut result = false;
        let lock = self.attempt.mutex.lock().unwrap();
        let data = self.data.lock().unwrap();
        if data.lazy_blocks_processed(hash) {
            result = true;
        } else {
            drop(data);
            drop(lock);
            if self
                .ledger
                .any()
                .block_exists_or_pruned(&self.ledger.read_txn(), hash)
            {
                result = true;
            }
        }
        result
    }

    pub fn lazy_batch_size(&self) -> u32 {
        let data = self.data.lock().unwrap();
        self.lazy_batch_size_locked(&data)
    }

    fn lazy_batch_size_locked(&self, data: &LazyData) -> u32 {
        let mut result = self.network_params.bootstrap.lazy_max_pull_blocks;
        let total_blocks = self.total_blocks();
        if total_blocks > bootstrap_limits::LAZY_BATCH_PULL_COUNT_RESIZE_BLOCKS_LIMIT
            && data.lazy_blocks_count != 0
        {
            let lazy_blocks_ratio = (total_blocks / data.lazy_blocks_count as u64) as f64;
            if lazy_blocks_ratio > bootstrap_limits::LAZY_BATCH_PULL_COUNT_RESIZE_RATIO {
                // Increasing blocks ratio weight as more important (^3). Small batch count should lower blocks ratio below target
                let lazy_blocks_factor = (lazy_blocks_ratio
                    / bootstrap_limits::LAZY_BATCH_PULL_COUNT_RESIZE_RATIO)
                    .powi(3);
                // Decreasing total block count weight as less important (sqrt)
                let total_blocks_factor = ((total_blocks
                    / bootstrap_limits::LAZY_BATCH_PULL_COUNT_RESIZE_BLOCKS_LIMIT)
                    as f64)
                    .sqrt();
                let batch_count_min = self.network_params.bootstrap.lazy_max_pull_blocks
                    / ((lazy_blocks_factor * total_blocks_factor) as u32);
                result = max(
                    self.network_params.bootstrap.lazy_min_pull_blocks,
                    batch_count_min,
                );
            }
        }
        result
    }

    pub fn lazy_start(&self, hash_or_account: &HashOrAccount) -> bool {
        let lock = self.attempt.mutex.lock().unwrap();
        let mut data = self.data.lock().unwrap();
        let mut inserted = false;
        // Add start blocks, limit 1024 (4k with disabled legacy bootstrap)
        let max_keys = if self.flags.disable_legacy_bootstrap {
            4 * 1024
        } else {
            1024
        };
        if data.lazy_keys.len() < max_keys
            && !data.lazy_keys.contains(&hash_or_account.into())
            && !data.lazy_blocks_processed(&hash_or_account.into())
        {
            data.lazy_keys.insert(hash_or_account.into());
            data.lazy_pulls.push_back((
                *hash_or_account,
                self.network_params.bootstrap.lazy_retry_limit,
            ));
            drop(data);
            drop(lock);
            self.attempt.condition.notify_all();
            inserted = true;
        }

        inserted
    }

    pub fn lazy_add(&self, pull: &PullInfo) {
        debug_assert_eq!(BlockHash::from(pull.account_or_head), pull.head);
        let _lock = self.attempt.mutex.lock().unwrap();
        let mut data = self.data.lock().unwrap();
        data.lazy_add(pull.account_or_head, pull.retry_limit);
    }

    pub fn lazy_requeue(&self, hash: &BlockHash, previous: &BlockHash) {
        let lock = self.attempt.mutex.lock().unwrap();
        let mut data = self.data.lock().unwrap();
        // Add only known blocks
        if data.lazy_blocks_processed(hash) {
            data.lazy_block_erase(hash);
            drop(data);
            drop(lock);
            self.connections.requeue_pull(
                PullInfo {
                    account_or_head: (*hash).into(),
                    head: *hash,
                    head_original: *hash,
                    end: *previous,
                    count: 1,
                    attempts: 0,
                    processed: 0,
                    retry_limit: self.network_params.bootstrap.lazy_destinations_retry_limit,
                    bootstrap_id: self.attempt.incremental_id,
                },
                false,
            );
        }
    }
}

impl Drop for BootstrapAttemptLazy {
    fn drop(&mut self) {
        let data = self.data.lock().unwrap();
        debug_assert_eq!(data.lazy_blocks.len(), data.lazy_blocks_count)
    }
}

impl BootstrapAttemptTrait for BootstrapAttemptLazy {
    fn incremental_id(&self) -> u64 {
        self.attempt.incremental_id
    }

    fn id(&self) -> &str {
        &self.attempt.id
    }

    fn started(&self) -> bool {
        self.attempt.started.load(Ordering::SeqCst)
    }

    fn stopped(&self) -> bool {
        self.attempt.stopped()
    }

    fn stop(&self) {
        self.attempt.stop()
    }

    fn pull_finished(&self) {
        self.attempt.pull_finished();
    }

    fn pulling(&self) -> u32 {
        self.attempt.pulling.load(Ordering::SeqCst)
    }

    fn total_blocks(&self) -> u64 {
        self.attempt.total_blocks.load(Ordering::SeqCst)
    }

    fn inc_total_blocks(&self) {
        self.attempt.total_blocks.fetch_add(1, Ordering::SeqCst);
    }

    fn requeued_pulls(&self) -> u32 {
        self.attempt.requeued_pulls.load(Ordering::SeqCst)
    }

    fn inc_requeued_pulls(&self) {
        self.attempt.requeued_pulls.fetch_add(1, Ordering::SeqCst);
    }

    fn pull_started(&self) {
        self.attempt.pull_started();
    }

    fn duration(&self) -> Duration {
        self.attempt.duration()
    }

    fn set_started(&self) -> bool {
        !self.attempt.started.swap(true, Ordering::SeqCst)
    }

    fn should_log(&self) -> bool {
        self.attempt.should_log()
    }

    fn notify(&self) {
        self.attempt.condition.notify_all();
    }

    fn get_information(&self, ptree: &mut dyn PropertyTree) -> anyhow::Result<()> {
        let data = self.data.lock().unwrap();
        ptree.put_u64("lazy_blocks", data.lazy_blocks.len() as u64)?;
        ptree.put_u64("lazy_state_backlog", data.lazy_state_backlog.len() as u64)?;
        ptree.put_u64("lazy_balances", data.lazy_balances.len() as u64)?;
        ptree.put_u64(
            "lazy_undefined_links",
            data.lazy_undefined_links.len() as u64,
        )?;
        ptree.put_u64("lazy_pulls", data.lazy_pulls.len() as u64)?;
        ptree.put_u64("lazy_keys", data.lazy_keys.len() as u64)?;
        if !data.lazy_keys.is_empty() {
            ptree.put_string(
                "lazy_key_1",
                &data.lazy_keys.iter().next().unwrap().to_string(),
            )?;
        }
        Ok(())
    }

    fn run(&self) {
        debug_assert!(self.started());
        debug_assert!(!self.flags.disable_lazy_bootstrap);
        self.connections.populate_connections(false);
        let mut lock = self.attempt.mutex.lock().unwrap();
        let mut data = self.data.lock().unwrap();
        data.lazy_start_time = Instant::now();
        while (self.attempt.still_pulling() || !data.lazy_finished(&self.attempt, &self.ledger))
            && !data.lazy_has_expired()
        {
            let mut iterations = 0u32;
            while self.attempt.still_pulling() && !data.lazy_has_expired() {
                while !(self.attempt.stopped()
                    || self.pulling() == 0
                    || (self.pulling()
                        < bootstrap_limits::BOOTSTRAP_CONNECTION_SCALE_TARGET_BLOCKS
                        && !data.lazy_pulls.is_empty())
                    || data.lazy_has_expired())
                {
                    drop(data);
                    lock = self.attempt.condition.wait(lock).unwrap();
                    data = self.data.lock().unwrap();
                }
                iterations += 1;
                // Flushing lazy pulls
                (lock, data) = self.lazy_pull_flush(lock, data);
                // Start backlog cleanup
                if iterations % 100 == 0 {
                    data.lazy_backlog_cleanup(&self.attempt, &self.ledger);
                }
            }
            // Flushing lazy pulls
            (lock, data) = self.lazy_pull_flush(lock, data);
            // Check if some blocks required for backlog were processed. Start destinations check
            if self.pulling() == 0 {
                data.lazy_backlog_cleanup(&self.attempt, &self.ledger);
                (lock, data) = self.lazy_pull_flush(lock, data);
            }
        }
        if !self.attempt.stopped() {
            debug!("Completed lazy pulls");
        }
        if data.lazy_has_expired() {
            debug!("Lazy bootstrap attempt ID {} expired", self.attempt.id);
        }
        drop(data);
        drop(lock);
        self.attempt.stop();
        self.attempt.condition.notify_all();
    }

    fn process_block(
        &self,
        block: Block,
        known_account: &Account,
        pull_blocks_processed: u64,
        max_blocks: u32,
        block_expected: bool,
        retry_limit: u32,
    ) -> bool {
        let stop_pull;
        if block_expected {
            stop_pull = self.process_block_lazy(
                block,
                known_account,
                pull_blocks_processed,
                max_blocks,
                retry_limit,
            );
        } else {
            // Drop connection with unexpected block for lazy bootstrap
            stop_pull = true;
        }
        stop_pull
    }
}
