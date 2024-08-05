use super::{ActiveElections, ActiveElectionsExt, ElectionBehavior};
use crate::{
    cementation::ConfirmingSet,
    config::NetworkConstants,
    stats::{DetailType, StatType, Stats},
};
use rsnano_core::{
    utils::{ContainerInfo, ContainerInfoComponent},
    Account, AccountInfo, ConfirmationHeightInfo,
};
use rsnano_ledger::Ledger;
use rsnano_store_lmdb::LmdbReadTransaction;
use std::{
    collections::{HashMap, VecDeque},
    mem::size_of,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Condvar, Mutex,
    },
    thread::JoinHandle,
    time::Instant,
};

#[derive(Clone)]
pub struct OptimisticSchedulerConfig {
    pub enabled: bool,

    /// Minimum difference between confirmation frontier and account frontier to become a candidate for optimistic confirmation
    pub gap_threshold: u64,

    /// Maximum number of candidates stored in memory
    pub max_size: usize,
}

impl OptimisticSchedulerConfig {
    pub fn new() -> Self {
        Self {
            enabled: true,
            gap_threshold: 32,
            max_size: 1024 * 64,
        }
    }
}

pub struct OptimisticScheduler {
    thread: Mutex<Option<JoinHandle<()>>>,
    config: OptimisticSchedulerConfig,
    stopped: AtomicBool,
    condition: Condvar,
    candidates: Mutex<OrderedCandidates>,
    stats: Arc<Stats>,
    active: Arc<ActiveElections>,
    network_constants: NetworkConstants,
    ledger: Arc<Ledger>,
    confirming_set: Arc<ConfirmingSet>,
}

impl OptimisticScheduler {
    pub fn new(
        config: OptimisticSchedulerConfig,
        stats: Arc<Stats>,
        active: Arc<ActiveElections>,
        network_constants: NetworkConstants,
        ledger: Arc<Ledger>,
        confirming_set: Arc<ConfirmingSet>,
    ) -> Self {
        Self {
            thread: Mutex::new(None),
            config,
            stopped: AtomicBool::new(false),
            condition: Condvar::new(),
            candidates: Mutex::new(OrderedCandidates::default()),
            stats,
            active,
            network_constants,
            ledger,
            confirming_set,
        }
    }

    pub fn stop(&self) {
        self.stopped.store(true, Ordering::SeqCst);
        self.notify();
        if let Some(handle) = self.thread.lock().unwrap().take() {
            handle.join().unwrap();
        }
    }

    /// Notify about changes in AEC vacancy
    pub fn notify(&self) {
        self.condition.notify_all();
    }

    fn activate_predicate(
        &self,
        account_info: &AccountInfo,
        conf_info: &ConfirmationHeightInfo,
    ) -> bool {
        let big_enough_gap =
            account_info.block_count - conf_info.height > self.config.gap_threshold;

        let nothing_confirmed_yet = conf_info.height == 0;

        big_enough_gap | nothing_confirmed_yet
    }

    /// Called from backlog population to process accounts with unconfirmed blocks
    pub fn activate(
        &self,
        account: &Account,
        account_info: &AccountInfo,
        conf_info: &ConfirmationHeightInfo,
    ) -> bool {
        if !self.config.enabled {
            return false;
        }

        if self.activate_predicate(account_info, conf_info) {
            {
                let mut candidates = self.candidates.lock().unwrap();
                // Prevent duplicate candidate accounts
                if candidates.contains(&account) {
                    return false; // Not activated
                }
                // Limit candidates container size
                if candidates.len() >= self.config.max_size {
                    return false; // Not activated
                }

                self.stats
                    .inc(StatType::OptimisticScheduler, DetailType::Activated);
                candidates.insert(*account, Instant::now());
            }
            true // Activated
        } else {
            false // Not activated
        }
    }

    pub fn collect_container_info(&self, name: impl Into<String>) -> ContainerInfoComponent {
        let guard = self.candidates.lock().unwrap();
        ContainerInfoComponent::Composite(
            name.into(),
            vec![ContainerInfoComponent::Leaf(ContainerInfo {
                name: "candidates".to_string(),
                count: guard.len(),
                sizeof_element: size_of::<Account>() * 2 + size_of::<Instant>(),
            })],
        )
    }

    fn predicate(&self, candidates: &OrderedCandidates) -> bool {
        if self.active.vacancy(ElectionBehavior::Optimistic) <= 0 {
            return false;
        }
        if let Some((_account, time)) = candidates.front() {
            time.elapsed() >= self.network_constants.optimistic_activation_delay
        } else {
            false
        }
    }

    fn run(&self) {
        let mut guard = self.candidates.lock().unwrap();
        while !self.stopped.load(Ordering::SeqCst) {
            self.stats
                .inc(StatType::OptimisticScheduler, DetailType::Loop);

            if self.predicate(&guard) {
                let tx = self.ledger.read_txn();

                while self.predicate(&guard) {
                    let (account, time) = guard.pop_front().unwrap();
                    drop(guard);
                    self.run_one(&tx, account, time);
                    guard = self.candidates.lock().unwrap();
                }
            }

            guard = self
                .condition
                .wait_timeout_while(
                    guard,
                    self.network_constants.optimistic_activation_delay / 2,
                    |g| !self.stopped.load(Ordering::SeqCst) && !self.predicate(g),
                )
                .unwrap()
                .0;
        }
    }

    fn run_one(&self, tx: &LmdbReadTransaction, account: Account, _time: Instant) {
        let any = self.ledger.any();
        let Some(head) = any.account_head(tx, &account) else {
            return;
        };
        if let Some(block) = any.get_block(tx, &head) {
            // Ensure block is not already confirmed
            if !self.confirming_set.exists(&block.hash())
                || self
                    .ledger
                    .confirmed()
                    .block_exists_or_pruned(tx, &block.hash())
            {
                // Try to insert it into AEC
                // We check for AEC vacancy inside our predicate
                let (inserted, _) =
                    self.active
                        .insert(&Arc::new(block), ElectionBehavior::Optimistic, None);
                self.stats.inc(
                    StatType::OptimisticScheduler,
                    if inserted {
                        DetailType::Insert
                    } else {
                        DetailType::InsertFailed
                    },
                );
            }
        }
    }
}

impl Drop for OptimisticScheduler {
    fn drop(&mut self) {
        // Thread must be stopped before destruction
        debug_assert!(self.thread.lock().unwrap().is_none())
    }
}

pub trait OptimisticSchedulerExt {
    fn start(&self);
}

impl OptimisticSchedulerExt for Arc<OptimisticScheduler> {
    fn start(&self) {
        debug_assert!(self.thread.lock().unwrap().is_none());
        if !self.config.enabled {
            return;
        }
        let self_l = Arc::clone(self);
        *self.thread.lock().unwrap() = Some(
            std::thread::Builder::new()
                .name("Sched Opt".to_string())
                .spawn(Box::new(move || {
                    self_l.run();
                }))
                .unwrap(),
        );
    }
}

#[derive(Default)]
struct OrderedCandidates {
    by_account: HashMap<Account, Instant>,
    sequenced: VecDeque<Account>,
}

impl OrderedCandidates {
    fn insert(&mut self, account: Account, time: Instant) {
        if let Some(_) = self.by_account.insert(account, time) {
            self.sequenced.retain(|i| *i != account);
        }
        self.sequenced.push_back(account);
    }

    fn len(&self) -> usize {
        self.sequenced.len()
    }

    fn contains(&self, account: &Account) -> bool {
        self.by_account.contains_key(account)
    }

    fn front(&self) -> Option<(Account, Instant)> {
        self.sequenced
            .front()
            .and_then(|account| self.by_account.get(account).map(|time| (*account, *time)))
    }

    fn pop_front(&mut self) -> Option<(Account, Instant)> {
        self.sequenced.pop_front().map(|account| {
            let time = self.by_account.remove(&account).unwrap();
            (account, time)
        })
    }
}
