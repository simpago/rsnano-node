use super::{ActiveElections, Bucket, BucketExt, PriorityBucketConfig};
use crate::stats::{DetailType, StatType, Stats};
use rsnano_core::{
    utils::ContainerInfo, Account, AccountInfo, Amount, ConfirmationHeightInfo, SavedBlock,
};
use rsnano_ledger::Ledger;
use rsnano_store_lmdb::{LmdbReadTransaction, Transaction};
use std::{
    cmp::max,
    sync::{Arc, Condvar, Mutex},
    thread::JoinHandle,
    time::Duration,
};
use tracing::trace;

pub struct PriorityScheduler {
    mutex: Mutex<PrioritySchedulerImpl>,
    condition: Condvar,
    ledger: Arc<Ledger>,
    stats: Arc<Stats>,
    buckets: Vec<Arc<Bucket>>,
    thread: Mutex<Option<JoinHandle<()>>>,
    cleanup_thread: Mutex<Option<JoinHandle<()>>>,
}

fn create_buckets(
    config: PriorityBucketConfig,
    active: Arc<ActiveElections>,
    stats: Arc<Stats>,
) -> Vec<Arc<Bucket>> {
    let mut buckets = Vec::new();
    let mut build_region = |begin: u128, end: u128, count: usize| {
        let width = (end - begin) / (count as u128);
        for i in 0..count {
            let minimum_balance = begin + (i as u128 * width);
            buckets.push(Arc::new(Bucket::new(
                minimum_balance.into(),
                config.clone(),
                active.clone(),
                stats.clone(),
            )))
        }
    };

    build_region(0, 1 << 79, 1);
    build_region(1 << 79, 1 << 88, 1);
    build_region(1 << 88, 1 << 92, 2);
    build_region(1 << 92, 1 << 96, 4);
    build_region(1 << 96, 1 << 100, 8);
    build_region(1 << 100, 1 << 104, 16);
    build_region(1 << 104, 1 << 108, 16);
    build_region(1 << 108, 1 << 112, 8);
    build_region(1 << 112, 1 << 116, 4);
    build_region(1 << 116, 1 << 120, 2);
    build_region(1 << 120, 1 << 127, 1);

    buckets
}

impl PriorityScheduler {
    pub(crate) fn new(
        config: PriorityBucketConfig,
        ledger: Arc<Ledger>,
        stats: Arc<Stats>,
        active: Arc<ActiveElections>,
    ) -> Self {
        Self {
            thread: Mutex::new(None),
            cleanup_thread: Mutex::new(None),
            mutex: Mutex::new(PrioritySchedulerImpl { stopped: false }),
            condition: Condvar::new(),
            buckets: create_buckets(config, active, stats.clone()),
            ledger,
            stats,
        }
    }

    pub fn stop(&self) {
        self.mutex.lock().unwrap().stopped = true;
        self.condition.notify_all();
        let handle = self.thread.lock().unwrap().take();
        if let Some(handle) = handle {
            handle.join().unwrap();
        }
        let handle = self.cleanup_thread.lock().unwrap().take();
        if let Some(handle) = handle {
            handle.join().unwrap();
        }
    }

    pub fn notify(&self) {
        self.condition.notify_all();
    }

    pub fn activate(&self, tx: &dyn Transaction, account: &Account) -> bool {
        debug_assert!(!account.is_zero());
        if let Some(account_info) = self.ledger.any().get_account(tx, account) {
            let conf_info = self
                .ledger
                .store
                .confirmation_height
                .get(tx, account)
                .unwrap_or_default();
            if conf_info.height < account_info.block_count {
                return self.activate_with_info(tx, account, &account_info, &conf_info);
            }
        };

        self.stats
            .inc(StatType::ElectionScheduler, DetailType::ActivateSkip);
        false // Not activated
    }

    pub fn activate_with_info(
        &self,
        tx: &dyn Transaction,
        account: &Account,
        account_info: &AccountInfo,
        conf_info: &ConfirmationHeightInfo,
    ) -> bool {
        debug_assert!(conf_info.frontier != account_info.head);

        let hash = match conf_info.height {
            0 => account_info.open_block,
            _ => self
                .ledger
                .any()
                .block_successor(tx, &conf_info.frontier)
                .unwrap(),
        };

        let Some(block) = self.ledger.any().get_block(tx, &hash) else {
            // Not activated
            return false;
        };

        if !self.ledger.dependents_confirmed(tx, &block) {
            self.stats
                .inc(StatType::ElectionScheduler, DetailType::ActivateFailed);
            return false; // Not activated
        }

        let balance = block.balance();
        let previous_balance = self
            .ledger
            .any()
            .block_balance(tx, &conf_info.frontier)
            .unwrap_or_default();
        let balance_priority = max(balance, previous_balance);

        let added = self
            .find_bucket(balance_priority)
            .push(account_info.modified, block.into());

        if added {
            self.stats
                .inc(StatType::ElectionScheduler, DetailType::Activated);
            trace!(
                account = account.encode_account(),
                time = account_info.modified,
                priority = ?balance_priority,
                "block activated"
            );
            self.condition.notify_all();
        } else {
            self.stats
                .inc(StatType::ElectionScheduler, DetailType::ActivateFull);
        }

        true // Activated
    }

    fn find_bucket(&self, priority: Amount) -> &Bucket {
        let mut result = &self.buckets[0];
        for bucket in &self.buckets[1..] {
            if bucket.can_accept(priority) {
                result = bucket;
            } else {
                break;
            }
        }
        result
    }

    pub fn len(&self) -> usize {
        self.buckets.iter().map(|b| b.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn predicate(&self) -> bool {
        self.buckets.iter().any(|b| b.available())
    }

    fn run(&self) {
        let mut guard = self.mutex.lock().unwrap();
        while !guard.stopped {
            guard = self
                .condition
                .wait_while(guard, |i| !i.stopped && !self.predicate())
                .unwrap();
            if !guard.stopped {
                drop(guard);
                self.stats
                    .inc(StatType::ElectionScheduler, DetailType::Loop);

                for bucket in &self.buckets {
                    if bucket.available() {
                        bucket.activate();
                    }
                }

                guard = self.mutex.lock().unwrap();
            }
        }
    }

    fn run_cleanup(&self) {
        let mut guard = self.mutex.lock().unwrap();
        while !guard.stopped {
            guard = self
                .condition
                .wait_timeout_while(guard, Duration::from_secs(1), |i| !i.stopped)
                .unwrap()
                .0;

            if !guard.stopped {
                drop(guard);
                self.stats
                    .inc(StatType::ElectionScheduler, DetailType::Cleanup);
                for bucket in &self.buckets {
                    bucket.update();
                }

                guard = self.mutex.lock().unwrap();
            }
        }
    }

    pub fn activate_successors(&self, tx: &LmdbReadTransaction, block: &SavedBlock) -> bool {
        let mut result = self.activate(tx, &block.account());

        // Start or vote for the next unconfirmed block in the destination account
        if let Some(destination) = block.destination() {
            if block.is_send() && !destination.is_zero() && destination != block.account() {
                result |= self.activate(tx, &destination);
            }
        }
        result
    }

    pub fn container_info(&self) -> ContainerInfo {
        let mut bucket_infos = ContainerInfo::builder();
        let mut election_infos = ContainerInfo::builder();

        for (id, bucket) in self.buckets.iter().enumerate() {
            bucket_infos = bucket_infos.leaf(id.to_string(), bucket.len(), 0);
            election_infos = election_infos.leaf(id.to_string(), bucket.election_count(), 0);
        }

        ContainerInfo::builder()
            .node("blocks", bucket_infos.finish())
            .node("elections", election_infos.finish())
            .finish()
    }
}

impl Drop for PriorityScheduler {
    fn drop(&mut self) {
        // Thread must be stopped before destruction
        debug_assert!(self.thread.lock().unwrap().is_none());
        debug_assert!(self.cleanup_thread.lock().unwrap().is_none());
    }
}

pub trait PrioritySchedulerExt {
    fn start(&self);
}

impl PrioritySchedulerExt for Arc<PriorityScheduler> {
    fn start(&self) {
        debug_assert!(self.thread.lock().unwrap().is_none());
        debug_assert!(self.cleanup_thread.lock().unwrap().is_none());

        let self_l = Arc::clone(&self);
        *self.thread.lock().unwrap() = Some(
            std::thread::Builder::new()
                .name("Sched Priority".to_string())
                .spawn(Box::new(move || {
                    self_l.run();
                }))
                .unwrap(),
        );

        let self_l = Arc::clone(&self);
        *self.cleanup_thread.lock().unwrap() = Some(
            std::thread::Builder::new()
                .name("Sched Priority".to_string())
                .spawn(Box::new(move || {
                    self_l.run_cleanup();
                }))
                .unwrap(),
        );
    }
}

struct PrioritySchedulerImpl {
    stopped: bool,
}
