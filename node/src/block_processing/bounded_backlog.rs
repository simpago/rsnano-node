use super::{backlog_index::BacklogIndex, BacklogScan, BlockProcessor};
use crate::{
    cementation::ConfirmingSet,
    stats::{DetailType, StatType, Stats},
    utils::{ThreadPool, ThreadPoolImpl},
};
use rsnano_core::BlockHash;
use rsnano_ledger::{Ledger, Writer};
use std::{
    cmp::min,
    ops::Deref,
    sync::{Arc, Condvar, Mutex, RwLock},
    thread::JoinHandle,
    time::Duration,
};

#[derive(Clone, Debug, PartialEq)]
pub struct BoundedBacklogConfig {
    pub max_backlog: usize,
    pub bucket_threshold: usize,
    pub overfill_factor: f64,
    pub batch_size: usize,
    pub max_queued_notifications: usize,
}

impl Default for BoundedBacklogConfig {
    fn default() -> Self {
        Self {
            max_backlog: 100_000,
            bucket_threshold: 1_000,
            overfill_factor: 1.5,
            batch_size: 32,
            max_queued_notifications: 128,
        }
    }
}

pub struct BoundedBacklog {
    thread: Mutex<Option<JoinHandle<()>>>,
    backlog_impl: Arc<BoundedBacklogImpl>,
}

impl BoundedBacklog {
    pub fn new(
        bucket_count: usize,
        config: BoundedBacklogConfig,
        ledger: Arc<Ledger>,
        backlog_scan: Arc<BacklogScan>,
        block_processor: Arc<BlockProcessor>,
        confirming_set: Arc<ConfirmingSet>,
        stats: Arc<Stats>,
    ) -> Self {
        let backlog_impl = Arc::new(BoundedBacklogImpl {
            condition: Condvar::new(),
            mutex: Mutex::new(BacklogData {
                stopped: false,
                index: BacklogIndex::new(bucket_count),
                ledger: ledger.clone(),
                config: config.clone(),
                bucket_count,
            }),
            workers: ThreadPoolImpl::create(1, "Bounded b notif".to_string()),
            config,
            stats,
            ledger,
            can_rollback: RwLock::new(Box::new(|_| true)),
        });

        Self {
            backlog_impl,
            thread: Mutex::new(None),
        }
    }

    pub fn start(&self) {
        //TODO debug_assert not running

        let backlog_impl = self.backlog_impl.clone();
        let handle = std::thread::Builder::new()
            .name("Bounded backlog".to_owned())
            .spawn(move || backlog_impl.run())
            .unwrap();
        *self.thread.lock().unwrap() = Some(handle);

        //TODO start scan thread
    }
    pub fn stop(&self) {
        todo!()
    }

    // Give other components a chance to veto a rollback
    pub fn on_rolling_back(&self, f: impl Fn(&BlockHash) -> bool + Send + Sync + 'static) {
        *self.backlog_impl.can_rollback.write().unwrap() = Box::new(f);
    }
}

struct BoundedBacklogImpl {
    mutex: Mutex<BacklogData>,
    condition: Condvar,
    workers: ThreadPoolImpl,
    config: BoundedBacklogConfig,
    stats: Arc<Stats>,
    ledger: Arc<Ledger>,
    can_rollback: RwLock<Box<dyn Fn(&BlockHash) -> bool + Send + Sync>>,
}

impl BoundedBacklogImpl {
    fn run(&self) {
        let mut guard = self.mutex.lock().unwrap();
        while !guard.stopped {
            if guard.predicate() {
                // Wait until all notification about the previous rollbacks are processed
                while self.workers.num_queued_tasks() >= self.config.max_queued_notifications {
                    self.stats
                        .inc(StatType::BoundedBacklog, DetailType::Cooldown);
                    guard = self
                        .condition
                        .wait_timeout_while(guard, Duration::from_millis(100), |i| !i.stopped)
                        .unwrap()
                        .0;
                    if guard.stopped {
                        return;
                    }
                }

                self.stats.inc(StatType::BoundedBacklog, DetailType::Loop);

                // Calculate the number of targets to rollback
                let backlog = self.ledger.backlog_count() as usize;

                let target_count = if backlog > self.config.max_backlog {
                    backlog - self.config.max_backlog
                } else {
                    0
                };

                let targets = guard.gather_targets(
                    min(target_count, self.config.batch_size),
                    &*self.can_rollback.read().unwrap(),
                );

                if !targets.is_empty() {
                    drop(guard);
                    self.stats.add(
                        StatType::BoundedBacklog,
                        DetailType::GatheredTargets,
                        targets.len() as u64,
                    );
                    let processed = self.perform_rollbacks(&targets);
                    guard = self.mutex.lock().unwrap();

                    // Erase rolled back blocks from the index
                    for hash in &processed {
                        guard.index.erase_hash(hash);
                    }
                } else {
                    // Cooldown, this should not happen in normal operation
                    self.stats
                        .inc(StatType::BoundedBacklog, DetailType::NoTargets);
                    guard = self
                        .condition
                        .wait_timeout_while(guard, Duration::from_millis(100), |i| !i.stopped)
                        .unwrap()
                        .0;
                }
            } else {
                guard = self
                    .condition
                    .wait_timeout_while(guard, Duration::from_secs(1), |i| {
                        !i.stopped && !i.predicate()
                    })
                    .unwrap()
                    .0;
            }
        }
    }

    fn perform_rollbacks(&self, targets: &[BlockHash]) -> Vec<BlockHash> {
        self.stats
            .inc(StatType::BoundedBacklog, DetailType::PerformingRollbacks);

        let _guard = self.ledger.write_queue.wait(Writer::BoundedBacklog);
        let tx = self.ledger.rw_txn();

        let mut processed = Vec::new();
        for hash in targets {
            // Skip the rollback if the block is being used by the node, this should be race free as it's checked while holding the ledger write lock
            if !(self.can_rollback.read().unwrap())(hash) {
                self.stats
                    .inc(StatType::BoundedBacklog, DetailType::RollbackSkipped);
                continue;
            }

            // TODO:

            //	// Here we check that the block is still OK to rollback, there could be a delay between gathering the targets and performing the rollbacks
            //	if (auto block = ledger.any.block_get (transaction, hash))
            //	{
            //		logger.debug (nano::log::type::bounded_backlog, "Rolling back: {}, account: {}", hash.to_string (), block->account ().to_account ());

            //		std::deque<std::shared_ptr<nano::block>> rollback_list;
            //		bool error = ledger.rollback (transaction, hash, rollback_list);
            //		stats.inc (nano::stat::type::bounded_backlog, error ? nano::stat::detail::rollback_failed : nano::stat::detail::rollback);

            //		for (auto const & rollback : rollback_list)
            //		{
            //			processed.push_back (rollback->hash ());
            //		}

            //		// Notify observers of the rolled back blocks on a background thread, avoid dispatching notifications when holding ledger write transaction
            //		workers.post ([this, rollback_list = std::move (rollback_list), root = block->qualified_root ()] {
            //			// TODO: Calling block_processor's event here is not ideal, but duplicating these events is even worse
            //			block_processor.rolled_back.notify (rollback_list, root);
            //		});
            //	}
            //	else
            //	{
            //		stats.inc (nano::stat::type::bounded_backlog, nano::stat::detail::rollback_missing_block);
            //		processed.push_back (hash);
            //	}
        }

        processed
    }
}

struct BacklogData {
    stopped: bool,
    index: BacklogIndex,
    ledger: Arc<Ledger>,
    config: BoundedBacklogConfig,
    bucket_count: usize,
}

impl BacklogData {
    fn predicate(&self) -> bool {
        // Both ledger and tracked backlog must be over the threshold
        self.ledger.backlog_count() as usize > self.config.max_backlog
            && self.index.len() > self.config.max_backlog
    }

    fn gather_targets(
        &self,
        max_count: usize,
        can_rollback: impl Fn(&BlockHash) -> bool,
    ) -> Vec<BlockHash> {
        let mut targets = Vec::new();

        // Start rolling back from lowest index buckets first
        for bucket in 0..self.bucket_count {
            // Only start rolling back if the bucket is over the threshold of unconfirmed blocks
            if self.index.len_of_bucket(bucket) > self.config.bucket_threshold {
                let count = min(max_count, self.config.batch_size);
                let top = self.index.top(bucket, count, |hash| {
                    // Only rollback if the block is not being used by the node
                    can_rollback(hash)
                });
                targets.extend(top);
            }
        }
        targets
    }
}
