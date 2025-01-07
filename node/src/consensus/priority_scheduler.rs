use super::{bucketing::Bucketing, ActiveElections, Bucket, BucketExt, PriorityBucketConfig};
use crate::stats::{DetailType, StatType, Stats};
use rsnano_core::{
    utils::ContainerInfo, Account, AccountInfo, Amount, BlockHash, ConfirmationHeightInfo,
    SavedBlock,
};
use rsnano_ledger::Ledger;
use rsnano_store_lmdb::{LmdbReadTransaction, Transaction};
use std::{
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
    bucketing: Bucketing,
    buckets: Vec<Arc<Bucket>>,
    thread: Mutex<Option<JoinHandle<()>>>,
    cleanup_thread: Mutex<Option<JoinHandle<()>>>,
}

impl PriorityScheduler {
    pub(crate) fn new(
        config: PriorityBucketConfig,
        ledger: Arc<Ledger>,
        stats: Arc<Stats>,
        active: Arc<ActiveElections>,
    ) -> Self {
        let bucketing = Bucketing::default();
        let mut buckets = Vec::with_capacity(bucketing.bucket_count());
        for _ in 0..bucketing.bucket_count() {
            buckets.push(Arc::new(Bucket::new(
                config.clone(),
                active.clone(),
                stats.clone(),
            )))
        }

        Self {
            thread: Mutex::new(None),
            cleanup_thread: Mutex::new(None),
            mutex: Mutex::new(PrioritySchedulerImpl { stopped: false }),
            condition: Condvar::new(),
            buckets,
            bucketing,
            ledger,
            stats,
        }
    }

    pub fn bucketing(&self) -> &Bucketing {
        &self.bucketing
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

    pub fn contains(&self, hash: &BlockHash) -> bool {
        self.buckets.iter().any(|b| b.contains(hash))
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

        let (priority_balance, priority_timestamp) = self.ledger.block_priority(tx, &block);

        let added = self
            .find_bucket(priority_balance)
            .push(priority_timestamp, block.into());

        if added {
            self.stats
                .inc(StatType::ElectionScheduler, DetailType::Activated);
            trace!(
                account = account.encode_account(),
                time = %account_info.modified,
                priority_balance = ?priority_balance,
                priority_timestamp = ?priority_timestamp,
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
        let index = self.bucketing.bucket_index(priority);
        &self.buckets[index]
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
