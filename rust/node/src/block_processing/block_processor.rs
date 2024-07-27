use super::UncheckedMap;
use crate::{
    config::BlockProcessorConfig,
    stats::{DetailType, StatType, Stats},
    transport::{ChannelEnum, FairQueue, Origin},
};
use rsnano_core::{
    utils::{ContainerInfo, ContainerInfoComponent},
    BlockEnum, BlockType, Epoch, HackyUnsafeMutBlock, HashOrAccount, UncheckedInfo,
};
use rsnano_ledger::{BlockStatus, Ledger, Writer};
use rsnano_store_lmdb::LmdbWriteTransaction;
use std::{
    collections::VecDeque,
    mem::size_of,
    sync::{Arc, Condvar, Mutex, MutexGuard},
    thread::JoinHandle,
    time::{Duration, Instant},
};
use tracing::{debug, error, info, trace};

#[derive(FromPrimitive, Copy, Clone, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum BlockSource {
    Unknown = 0,
    Live,
    LiveOriginator,
    Bootstrap,
    BootstrapLegacy,
    Unchecked,
    Local,
    Forced,
}

impl From<BlockSource> for DetailType {
    fn from(value: BlockSource) -> Self {
        match value {
            BlockSource::Unknown => DetailType::Unknown,
            BlockSource::Live => DetailType::Live,
            BlockSource::LiveOriginator => DetailType::LiveOriginator,
            BlockSource::Bootstrap => DetailType::Bootstrap,
            BlockSource::BootstrapLegacy => DetailType::BootstrapLegacy,
            BlockSource::Unchecked => DetailType::Unchecked,
            BlockSource::Local => DetailType::Local,
            BlockSource::Forced => DetailType::Forced,
        }
    }
}

pub struct BlockProcessorContext {
    pub block: Arc<BlockEnum>,
    pub source: BlockSource,
    pub arrival: Instant,
    waiter: Arc<BlockProcessorWaiter>,
}

impl BlockProcessorContext {
    pub fn new(block: Arc<BlockEnum>, source: BlockSource) -> Self {
        Self {
            block,
            source,
            arrival: Instant::now(),
            waiter: Arc::new(BlockProcessorWaiter::new()),
        }
    }

    pub fn set_result(&self, result: BlockStatus) {
        self.waiter.set_result(result);
    }

    pub fn get_waiter(&self) -> Arc<BlockProcessorWaiter> {
        self.waiter.clone()
    }
}

impl Drop for BlockProcessorContext {
    fn drop(&mut self) {
        self.waiter.cancel()
    }
}

pub struct BlockProcessorWaiter {
    result: Mutex<(Option<BlockStatus>, bool)>, // (status, done)
    condition: Condvar,
}

impl BlockProcessorWaiter {
    pub fn new() -> Self {
        Self {
            result: Mutex::new((None, false)),
            condition: Condvar::new(),
        }
    }

    pub fn set_result(&self, result: BlockStatus) {
        *self.result.lock().unwrap() = (Some(result), true);
        self.condition.notify_all();
    }

    pub fn cancel(&self) {
        self.result.lock().unwrap().1 = true;
        self.condition.notify_all();
    }

    pub fn wait_result(&self) -> Option<BlockStatus> {
        let guard = self.result.lock().unwrap();
        if guard.1 {
            return guard.0;
        }

        self.condition.wait_while(guard, |i| !i.1).unwrap().0
    }
}

pub struct BlockProcessor {
    thread: Mutex<Option<JoinHandle<()>>>,
    processor_loop: Arc<BlockProcessorLoop>,
}

impl BlockProcessor {
    pub fn new(
        config: BlockProcessorConfig,
        ledger: Arc<Ledger>,
        unchecked_map: Arc<UncheckedMap>,
        stats: Arc<Stats>,
    ) -> Self {
        let config_l = config.clone();
        let max_size_query = Box::new(move |origin: &Origin<BlockSource>| match origin.source {
            BlockSource::Live | BlockSource::LiveOriginator => config_l.max_peer_queue,
            _ => config_l.max_system_queue,
        });

        let config_l = config.clone();
        let priority_query = Box::new(move |origin: &Origin<BlockSource>| match origin.source {
            BlockSource::Live | BlockSource::LiveOriginator => config.priority_live,
            BlockSource::Bootstrap | BlockSource::BootstrapLegacy | BlockSource::Unchecked => {
                config_l.priority_bootstrap
            }
            BlockSource::Local => config_l.priority_local,
            _ => 1,
        });

        Self {
            processor_loop: Arc::new(BlockProcessorLoop {
                mutex: Mutex::new(BlockProcessorImpl {
                    queue: FairQueue::new(max_size_query, priority_query),
                    last_log: None,
                    stopped: false,
                }),
                condition: Condvar::new(),
                ledger,
                unchecked_map,
                config,
                stats,
                blocks_rolled_back: Mutex::new(None),
                block_rolled_back: Mutex::new(Vec::new()),
                block_processed: Mutex::new(Vec::new()),
                batch_processed: Mutex::new(Vec::new()),
            }),
            thread: Mutex::new(None),
        }
    }

    pub fn new_test_instance(ledger: Arc<Ledger>) -> Self {
        BlockProcessor::new(
            BlockProcessorConfig::default(),
            ledger,
            Arc::new(UncheckedMap::default()),
            Arc::new(Stats::default()),
        )
    }

    pub fn new_null() -> Self {
        Self::new_test_instance(Arc::new(Ledger::new_null()))
    }

    pub fn start(&self) {
        debug_assert!(self.thread.lock().unwrap().is_none());
        let processor_loop = Arc::clone(&self.processor_loop);
        *self.thread.lock().unwrap() = Some(
            std::thread::Builder::new()
                .name("Blck processing".to_string())
                .spawn(move || {
                    processor_loop.run();
                })
                .unwrap(),
        );
    }

    pub fn stop(&self) {
        self.processor_loop.mutex.lock().unwrap().stopped = true;
        self.processor_loop.condition.notify_all();
        if let Some(join_handle) = self.thread.lock().unwrap().take() {
            join_handle.join().unwrap();
        }
    }

    pub fn total_queue_len(&self) -> usize {
        self.processor_loop.total_queue_len()
    }

    pub fn queue_len(&self, source: BlockSource) -> usize {
        self.processor_loop.queue_len(source)
    }

    pub fn add_block_processed_observer(
        &self,
        observer: Box<dyn Fn(BlockStatus, &BlockProcessorContext) + Send + Sync>,
    ) {
        self.processor_loop.add_block_processed_observer(observer);
    }

    pub fn add_batch_processed_observer(
        &self,
        observer: Box<dyn Fn(&[(BlockStatus, Arc<BlockProcessorContext>)]) + Send + Sync>,
    ) {
        self.processor_loop.add_batch_processed_observer(observer);
    }

    pub fn add_rolled_back_observer(&self, observer: Box<dyn Fn(&BlockEnum) + Send + Sync>) {
        self.processor_loop.add_rolled_back_observer(observer);
    }

    pub fn add(
        &self,
        block: Arc<BlockEnum>,
        source: BlockSource,
        channel: Option<Arc<ChannelEnum>>,
    ) -> bool {
        self.processor_loop.add(block, source, channel)
    }

    pub fn add_blocking(&self, block: Arc<BlockEnum>, source: BlockSource) -> Option<BlockStatus> {
        self.processor_loop.add_blocking(block, source)
    }

    pub fn process_active(&self, block: Arc<BlockEnum>) {
        self.processor_loop.process_active(block);
    }

    pub fn notify_block_rolled_back(&self, block: &BlockEnum) {
        self.processor_loop.notify_block_rolled_back(block)
    }

    pub fn set_blocks_rolled_back_callback(
        &self,
        callback: Box<dyn Fn(Vec<BlockEnum>, BlockEnum) + Send + Sync>,
    ) {
        self.processor_loop
            .set_blocks_rolled_back_callback(callback);
    }
    pub fn force(&self, block: Arc<BlockEnum>) {
        self.processor_loop.force(block);
    }

    pub fn collect_container_info(&self, name: impl Into<String>) -> ContainerInfoComponent {
        self.processor_loop.collect_container_info(name)
    }
}

impl Drop for BlockProcessor {
    fn drop(&mut self) {
        // Thread must be stopped before destruction
        debug_assert!(self.thread.lock().unwrap().is_none());
    }
}

struct BlockProcessorLoop {
    mutex: Mutex<BlockProcessorImpl>,
    condition: Condvar,
    ledger: Arc<Ledger>,
    unchecked_map: Arc<UncheckedMap>,
    config: BlockProcessorConfig,
    stats: Arc<Stats>,
    blocks_rolled_back: Mutex<Option<Box<dyn Fn(Vec<BlockEnum>, BlockEnum) + Send + Sync>>>,
    block_rolled_back: Mutex<Vec<Box<dyn Fn(&BlockEnum) + Send + Sync>>>,
    block_processed: Mutex<Vec<Box<dyn Fn(BlockStatus, &BlockProcessorContext) + Send + Sync>>>,
    batch_processed:
        Mutex<Vec<Box<dyn Fn(&[(BlockStatus, Arc<BlockProcessorContext>)]) + Send + Sync>>>,
}

impl BlockProcessorLoop {
    pub fn run(&self) {
        let mut guard = self.mutex.lock().unwrap();
        while !guard.stopped {
            if !guard.queue.is_empty() {
                if guard.should_log() {
                    info!(
                        "{} blocks (+ {} forced) in processing_queue",
                        guard.queue.len(),
                        guard
                            .queue
                            .queue_len(&Origin::new_opt(BlockSource::Forced, None))
                    );
                }

                let mut processed = self.process_batch(guard);

                // Set results for futures when not holding the lock
                for (result, context) in processed.iter_mut() {
                    context.set_result(*result);
                }

                self.notify_batch_processed(&processed);

                guard = self.mutex.lock().unwrap();
            } else {
                self.condition.notify_one();
                guard = self.condition.wait(guard).unwrap();
            }
        }
    }

    fn notify_batch_processed(&self, blocks: &Vec<(BlockStatus, Arc<BlockProcessorContext>)>) {
        {
            let guard = self.block_processed.lock().unwrap();
            for observer in guard.iter() {
                for (status, context) in blocks {
                    observer(*status, context);
                }
            }
        }
        {
            let guard = self.batch_processed.lock().unwrap();
            for observer in guard.iter() {
                observer(&blocks);
            }
        }
    }

    pub fn add_block_processed_observer(
        &self,
        observer: Box<dyn Fn(BlockStatus, &BlockProcessorContext) + Send + Sync>,
    ) {
        self.block_processed.lock().unwrap().push(observer);
    }

    pub fn add_batch_processed_observer(
        &self,
        observer: Box<dyn Fn(&[(BlockStatus, Arc<BlockProcessorContext>)]) + Send + Sync>,
    ) {
        self.batch_processed.lock().unwrap().push(observer);
    }

    pub fn add_rolled_back_observer(&self, observer: Box<dyn Fn(&BlockEnum) + Send + Sync>) {
        self.block_rolled_back.lock().unwrap().push(observer);
    }

    pub fn notify_block_rolled_back(&self, block: &BlockEnum) {
        for observer in self.block_rolled_back.lock().unwrap().iter() {
            observer(block)
        }
    }

    pub fn set_blocks_rolled_back_callback(
        &self,
        callback: Box<dyn Fn(Vec<BlockEnum>, BlockEnum) + Send + Sync>,
    ) {
        *self.blocks_rolled_back.lock().unwrap() = Some(callback);
    }

    pub fn process_active(&self, block: Arc<BlockEnum>) {
        self.add(block, BlockSource::Live, None);
    }

    pub fn add(
        &self,
        block: Arc<BlockEnum>,
        source: BlockSource,
        channel: Option<Arc<ChannelEnum>>,
    ) -> bool {
        if self.config.work_thresholds.validate_entry_block(&block) {
            // true => error
            self.stats
                .inc(StatType::Blockprocessor, DetailType::InsufficientWork);
            return false; // Not added
        }

        self.stats
            .inc(StatType::Blockprocessor, DetailType::Process);
        debug!(
            "Processing block (async): {} (source: {:?} {})",
            block.hash(),
            source,
            channel
                .as_ref()
                .map(|c| c.remote_endpoint().to_string())
                .unwrap_or_else(|| "<unknown>".to_string())
        );

        self.add_impl(Arc::new(BlockProcessorContext::new(block, source)), channel)
    }

    pub fn add_blocking(&self, block: Arc<BlockEnum>, source: BlockSource) -> Option<BlockStatus> {
        self.stats
            .inc(StatType::Blockprocessor, DetailType::ProcessBlocking);
        debug!(
            "Processing block (blocking): {} (source: {:?})",
            block.hash(),
            source
        );

        let hash = block.hash();
        let ctx = Arc::new(BlockProcessorContext::new(block, source));
        let waiter = ctx.get_waiter();
        self.add_impl(ctx, None);

        match waiter.wait_result() {
            Some(status) => Some(status),
            None => {
                self.stats
                    .inc(StatType::Blockprocessor, DetailType::ProcessBlockingTimeout);
                error!("Block dropped when processing: {}", hash);
                None
            }
        }
    }

    pub fn force(&self, block: Arc<BlockEnum>) {
        self.stats.inc(StatType::Blockprocessor, DetailType::Force);
        debug!("Forcing block: {}", block.hash());
        let ctx = Arc::new(BlockProcessorContext::new(block, BlockSource::Forced));
        self.add_impl(ctx, None);
    }

    // TODO: Remove and replace all checks with calls to size (block_source)
    pub fn total_queue_len(&self) -> usize {
        self.mutex.lock().unwrap().queue.len()
    }

    pub fn queue_len(&self, source: BlockSource) -> usize {
        self.mutex.lock().unwrap().queue.queue_len(&source.into())
    }

    fn add_impl(
        &self,
        context: Arc<BlockProcessorContext>,
        channel: Option<Arc<ChannelEnum>>,
    ) -> bool {
        let source = context.source;
        let added;
        {
            let mut guard = self.mutex.lock().unwrap();
            added = guard.queue.push(context, Origin::new_opt(source, channel));
        }
        if added {
            self.condition.notify_all();
        } else {
            self.stats
                .inc(StatType::Blockprocessor, DetailType::Overfill);
            self.stats
                .inc(StatType::BlockprocessorOverfill, source.into());
        }
        added
    }

    pub fn queue_unchecked(&self, hash_or_account: &HashOrAccount) {
        self.unchecked_map.trigger(hash_or_account);
    }

    fn next_batch(
        &self,
        data: &mut BlockProcessorImpl,
        max_count: usize,
    ) -> VecDeque<Arc<BlockProcessorContext>> {
        data.queue.periodic_update(Duration::from_secs(30));
        let mut results = VecDeque::new();
        while !data.queue.is_empty() && results.len() < max_count {
            results.push_back(data.next());
        }
        results
    }

    pub fn process_batch(
        &self,
        mut guard: MutexGuard<BlockProcessorImpl>,
    ) -> Vec<(BlockStatus, Arc<BlockProcessorContext>)> {
        let batch = self.next_batch(&mut guard, 256);
        drop(guard);

        let mut write_guard = self.ledger.write_queue.wait(Writer::BlockProcessor);
        let mut tx = self.ledger.rw_txn();

        let timer = Instant::now();

        // Processing blocks
        let mut number_of_blocks_processed = 0;
        let mut number_of_forced_processed = 0;

        let mut processed = Vec::new();
        for ctx in batch {
            let force = ctx.source == BlockSource::Forced;

            (write_guard, tx) = self.ledger.refresh_if_needed(write_guard, tx);

            if force {
                number_of_forced_processed += 1;
                self.rollback_competitor(&mut tx, &ctx.block);
            }

            number_of_blocks_processed += 1;

            let result = self.process_one(&mut tx, &ctx);
            processed.push((result, ctx));
        }

        if number_of_blocks_processed != 0 && timer.elapsed() > Duration::from_millis(100) {
            debug!(
                "Processed {} blocks ({} blocks were forced) in {} ms",
                number_of_blocks_processed,
                number_of_forced_processed,
                timer.elapsed().as_millis(),
            );
        }
        processed
    }

    pub fn process_one(
        &self,
        txn: &mut LmdbWriteTransaction,
        context: &BlockProcessorContext,
    ) -> BlockStatus {
        let block = &context.block;
        let hash = block.hash();
        let mutable_block = unsafe { block.undefined_behavior_mut() };

        let result = match self.ledger.process(txn, mutable_block) {
            Ok(()) => BlockStatus::Progress,
            Err(r) => r,
        };

        self.stats
            .inc(StatType::BlockprocessorResult, result.into());
        self.stats
            .inc(StatType::BlockprocessorSource, context.source.into());
        trace!(?result, block = %block.hash(), source = ?context.source, "Block processed");

        match result {
            BlockStatus::Progress => {
                self.queue_unchecked(&hash.into());
                /* For send blocks check epoch open unchecked (gap pending).
                For state blocks check only send subtype and only if block epoch is not last epoch.
                If epoch is last, then pending entry shouldn't trigger same epoch open block for destination account. */
                if block.block_type() == BlockType::LegacySend
                    || block.block_type() == BlockType::State
                        && block.is_send()
                        && block.sideband().unwrap().details.epoch < Epoch::MAX
                {
                    /* block->destination () for legacy send blocks
                    block->link () for state blocks (send subtype) */
                    self.queue_unchecked(&block.destination_or_link().into());
                }
            }
            BlockStatus::GapPrevious => {
                self.unchecked_map.put(
                    block.previous().into(),
                    UncheckedInfo::new(Arc::clone(block)),
                );
                self.stats.inc(StatType::Ledger, DetailType::GapPrevious);
            }
            BlockStatus::GapSource => {
                self.unchecked_map.put(
                    block
                        .source_field()
                        .unwrap_or(block.link_field().unwrap_or_default().into())
                        .into(),
                    UncheckedInfo::new(Arc::clone(block)),
                );
                self.stats.inc(StatType::Ledger, DetailType::GapSource);
            }
            BlockStatus::GapEpochOpenPending => {
                // Specific unchecked key starting with epoch open block account public key
                self.unchecked_map.put(
                    block.account().into(),
                    UncheckedInfo::new(Arc::clone(block)),
                );
                self.stats.inc(StatType::Ledger, DetailType::GapSource);
            }
            BlockStatus::Old => {
                self.stats.inc(StatType::Ledger, DetailType::Old);
            }
            BlockStatus::BadSignature => {}
            BlockStatus::NegativeSpend => {}
            BlockStatus::Unreceivable => {}
            BlockStatus::Fork => {
                self.stats.inc(StatType::Ledger, DetailType::Fork);
            }
            BlockStatus::OpenedBurnAccount => {}
            BlockStatus::BalanceMismatch => {}
            BlockStatus::RepresentativeMismatch => {}
            BlockStatus::BlockPosition => {}
            BlockStatus::InsufficientWork => {}
        }

        result
    }

    fn rollback_competitor(&self, transaction: &mut LmdbWriteTransaction, block: &Arc<BlockEnum>) {
        let hash = block.hash();
        if let Some(successor) = self
            .ledger
            .any()
            .block_successor_by_qualified_root(transaction, &block.qualified_root())
        {
            let successor_block = self
                .ledger
                .any()
                .get_block(transaction, &successor)
                .unwrap();
            if successor != hash {
                // Replace our block with the winner and roll back any dependent blocks
                debug!("Rolling back: {} and replacing with: {}", successor, hash);
                let rollback_list = match self.ledger.rollback(transaction, &successor) {
                    Ok(rollback_list) => {
                        self.stats.inc(StatType::Ledger, DetailType::Rollback);
                        debug!("Blocks rolled back: {}", rollback_list.len());
                        rollback_list
                    }
                    Err(_) => {
                        self.stats.inc(StatType::Ledger, DetailType::RollbackFailed);
                        error!(
                            "Failed to roll back: {} because it or a successor was confirmed",
                            successor
                        );
                        Vec::new()
                    }
                };

                let callback_guard = self.blocks_rolled_back.lock().unwrap();
                if let Some(callback) = callback_guard.as_ref() {
                    callback(rollback_list, successor_block);
                }
            }
        }
    }

    pub fn collect_container_info(&self, name: impl Into<String>) -> ContainerInfoComponent {
        let guard = self.mutex.lock().unwrap();
        ContainerInfoComponent::Composite(
            name.into(),
            vec![
                ContainerInfoComponent::Leaf(ContainerInfo {
                    name: "blocks".to_owned(),
                    count: guard.queue.len(),
                    sizeof_element: size_of::<Arc<BlockEnum>>(),
                }),
                ContainerInfoComponent::Leaf(ContainerInfo {
                    name: "forced".to_owned(),
                    count: guard.queue.queue_len(&BlockSource::Forced.into()),
                    sizeof_element: size_of::<Arc<BlockEnum>>(),
                }),
                guard.queue.collect_container_info("queue"),
            ],
        )
    }
}

struct BlockProcessorImpl {
    pub queue: FairQueue<Arc<BlockProcessorContext>, BlockSource>,
    pub last_log: Option<Instant>,
    stopped: bool,
}

impl BlockProcessorImpl {
    fn next(&mut self) -> Arc<BlockProcessorContext> {
        debug_assert!(!self.queue.is_empty()); // This should be checked before calling next
        if !self.queue.is_empty() {
            let (request, origin) = self.queue.next().unwrap();
            assert!(origin.source != BlockSource::Forced || request.source == BlockSource::Forced);
            return request;
        }

        panic!("next() called when no blocks are ready");
    }

    pub fn should_log(&mut self) -> bool {
        if let Some(last) = &self.last_log {
            if last.elapsed() >= Duration::from_secs(15) {
                self.last_log = Some(Instant::now());
                return true;
            }
        }

        false
    }
}
