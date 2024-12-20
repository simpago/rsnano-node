use super::{BacklogScan, BlockProcessor};
use crate::{cementation::ConfirmingSet, stats::Stats, utils::ThreadPoolImpl};
use rsnano_ledger::Ledger;
use std::{
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
    config: BoundedBacklogConfig,
    ledger: Arc<Ledger>,
    backlog_scan: Arc<BacklogScan>,
    block_processor: Arc<BlockProcessor>,
    confirming_set: Arc<ConfirmingSet>,
    stats: Arc<Stats>,
    thread: Mutex<Option<JoinHandle<()>>>,
    workers: ThreadPoolImpl,
}

impl BoundedBacklog {
    pub fn start(&self) {
        //TODO debug_assert not running

        let mut backlog_impl = BoundedBacklogImpl {
            condition: Condvar::new(),
            mutex: Mutex::new(BacklogData { stopped: false }),
        };
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
}

struct BoundedBacklogImpl {
    mutex: Mutex<BacklogData>,
    condition: Condvar,
}

impl BoundedBacklogImpl {
    fn run(&mut self) {
        let mut guard = self.mutex.lock().unwrap();
        while !guard.stopped {
            if guard.predicate() {
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
}

impl BacklogData {
    fn predicate(&self) -> bool {
        // Both ledger and tracked backlog must be over the threshold
        todo!()
    }
}
