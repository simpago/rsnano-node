use super::{backlog_index::BacklogIndex, BacklogScan, BlockProcessor};
use crate::{
    cementation::ConfirmingSet,
    stats::{DetailType, StatType, Stats},
    utils::{ThreadPool, ThreadPoolImpl},
};
use rsnano_core::BlockHash;
use rsnano_ledger::Ledger;
use std::{
    cmp::min,
    sync::{Arc, Condvar, Mutex},
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
                can_rollback: Box::new(|_| true),
            }),
            workers: ThreadPoolImpl::create(1, "Bounded b notif".to_string()),
            config,
            stats,
            ledger,
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
    pub fn on_rolling_back(&self, f: impl Fn(&BlockHash) -> bool + Send + 'static) {
        self.backlog_impl.mutex.lock().unwrap().can_rollback = Box::new(f);
    }
}

struct BoundedBacklogImpl {
    mutex: Mutex<BacklogData>,
    condition: Condvar,
    workers: ThreadPoolImpl,
    config: BoundedBacklogConfig,
    stats: Arc<Stats>,
    ledger: Arc<Ledger>,
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

                let targets = guard.gather_targets(min(target_count, self.config.batch_size));

                todo!()
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
}

struct BacklogData {
    stopped: bool,
    index: BacklogIndex,
    ledger: Arc<Ledger>,
    config: BoundedBacklogConfig,
    bucket_count: usize,
    can_rollback: Box<dyn Fn(&BlockHash) -> bool + Send>,
}

impl BacklogData {
    fn predicate(&self) -> bool {
        // Both ledger and tracked backlog must be over the threshold
        self.ledger.backlog_count() as usize > self.config.max_backlog
            && self.index.len() > self.config.max_backlog
    }

    fn gather_targets(&self, max_count: usize) -> Vec<BlockHash> {
        let mut targets = Vec::new();

        // Start rolling back from lowest index buckets first
        for bucket in 0..self.bucket_count {
            // Only start rolling back if the bucket is over the threshold of unconfirmed blocks
            if self.index.len_of_bucket(bucket) > self.config.bucket_threshold {
                let count = min(max_count, self.config.batch_size);
                let top = self.index.top(bucket, count, |hash| {
                    // Only rollback if the block is not being used by the node
                    (self.can_rollback)(hash)
                });
                targets.extend(top);
            }
        }
        targets
    }
}
