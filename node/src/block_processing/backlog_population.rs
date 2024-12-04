use crate::{
    consensus::election_schedulers::ElectionSchedulers,
    stats::{DetailType, StatType, Stats},
};
use primitive_types::U256;
use rsnano_core::{Account, AccountInfo};
use rsnano_ledger::Ledger;
use rsnano_store_lmdb::Transaction;
use std::{
    sync::{Arc, Condvar, Mutex},
    thread::{self, JoinHandle},
    time::Duration,
};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BacklogPopulationConfig {
    /** Control if ongoing backlog population is enabled. If not, backlog population can still be triggered by RPC */
    pub enabled: bool,

    /** Number of accounts per second to process. Number of accounts per single batch is this value divided by `frequency` */
    pub batch_size: u32,

    /** Number of batches to run per second. Batches run in 1 second / `frequency` intervals */
    pub frequency: u32,
}

impl Default for BacklogPopulationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            batch_size: 10 * 1000,
            frequency: 10,
        }
    }
}

struct BacklogPopulationFlags {
    stopped: bool,
    /** This is a manual trigger, the ongoing backlog population does not use this.
     *  It can be triggered even when backlog population (frontiers confirmation) is disabled. */
    triggered: bool,
}

pub struct BacklogPopulation {
    ledger: Arc<Ledger>,
    stats: Arc<Stats>,
    /**
     * Callback called for each backlogged account
     */
    activate_callback: Arc<Mutex<Option<ActivateCallback>>>,
    config: BacklogPopulationConfig,
    mutex: Arc<Mutex<BacklogPopulationFlags>>,
    condition: Arc<Condvar>,
    /** Thread that runs the backlog implementation logic. The thread always runs, even if
     *  backlog population is disabled, so that it can service a manual trigger (e.g. via RPC). */
    thread: Mutex<Option<JoinHandle<()>>>,
    election_schedulers: Arc<ElectionSchedulers>,
}

pub type ActivateCallback = Box<dyn Fn(&dyn Transaction, &Account) + Send + Sync>;

impl BacklogPopulation {
    pub(crate) fn new(
        config: BacklogPopulationConfig,
        ledger: Arc<Ledger>,
        stats: Arc<Stats>,
        election_schedulers: Arc<ElectionSchedulers>,
    ) -> Self {
        Self {
            config,
            ledger,
            stats,
            activate_callback: Arc::new(Mutex::new(None)),
            mutex: Arc::new(Mutex::new(BacklogPopulationFlags {
                stopped: false,
                triggered: false,
            })),
            condition: Arc::new(Condvar::new()),
            thread: Mutex::new(None),
            election_schedulers,
        }
    }

    pub fn set_activate_callback(&self, callback: ActivateCallback) {
        let mut lock = self.activate_callback.lock().unwrap();
        *lock = Some(callback);
    }

    pub fn start(&self) {
        debug_assert!(self.thread.lock().unwrap().is_none());

        let thread = BacklogPopulationThread {
            ledger: self.ledger.clone(),
            stats: self.stats.clone(),
            activate_callback: self.activate_callback.clone(),
            config: self.config.clone(),
            mutex: self.mutex.clone(),
            condition: self.condition.clone(),
            election_schedulers: self.election_schedulers.clone(),
        };

        *self.thread.lock().unwrap() = Some(
            thread::Builder::new()
                .name("Backlog".to_owned())
                .spawn(move || {
                    thread.run();
                })
                .unwrap(),
        );
    }

    pub fn stop(&self) {
        let mut lock = self.mutex.lock().unwrap();
        lock.stopped = true;
        drop(lock);
        self.notify();
        let handle = self.thread.lock().unwrap().take();
        if let Some(handle) = handle {
            handle.join().unwrap()
        }
    }

    /** Manually trigger backlog population */
    pub fn trigger(&self) {
        {
            let mut lock = self.mutex.lock().unwrap();
            lock.triggered = true;
        }
        self.notify();
    }

    /** Notify about AEC vacancy */
    pub fn notify(&self) {
        self.condition.notify_all();
    }
}

impl Drop for BacklogPopulation {
    fn drop(&mut self) {
        self.stop();
    }
}

struct BacklogPopulationThread {
    ledger: Arc<Ledger>,
    stats: Arc<Stats>,
    activate_callback: Arc<Mutex<Option<ActivateCallback>>>,
    config: BacklogPopulationConfig,
    mutex: Arc<Mutex<BacklogPopulationFlags>>,
    condition: Arc<Condvar>,
    election_schedulers: Arc<ElectionSchedulers>,
}

impl BacklogPopulationThread {
    fn run(&self) {
        let mut lock = self.mutex.lock().unwrap();
        while !lock.stopped {
            if self.predicate(&lock) {
                self.stats.inc(StatType::Backlog, DetailType::Loop);

                lock.triggered = false;
                drop(lock);
                self.populate_backlog();
                lock = self.mutex.lock().unwrap();
            }

            lock = self
                .condition
                .wait_while(lock, |l| !l.stopped && !self.predicate(l))
                .unwrap();
        }
    }

    fn predicate(&self, lock: &BacklogPopulationFlags) -> bool {
        lock.triggered || self.config.enabled
    }

    fn populate_backlog(&self) {
        debug_assert!(self.config.frequency > 0);
        let mut lock = self.mutex.lock().unwrap();

        let chunk_size = self.config.batch_size / self.config.frequency;
        let mut done = false;
        let mut next = Account::zero();
        while !lock.stopped && !done {
            drop(lock);
            {
                let mut transaction = self.ledger.store.tx_begin_read();

                let mut count = 0u32;
                let mut it = self.ledger.any().accounts_range(&transaction, next..);
                while let Some((account, info)) = it.next() {
                    if count >= chunk_size {
                        break;
                    }
                    if transaction.is_refresh_needed_with(Duration::from_millis(100)) {
                        drop(it);
                        transaction.refresh();
                        it = self.ledger.any().accounts_range(&transaction, account..);
                    }

                    self.stats.inc(StatType::Backlog, DetailType::Total);

                    self.activate(&transaction, &account, &info);
                    next = (account.number().overflowing_add(U256::from(1)).0).into();

                    count += 1;
                }
                done = next == Account::zero()
                    || self
                        .ledger
                        .any()
                        .accounts_range(&transaction, next..)
                        .next()
                        .is_none();
            }
            lock = self.mutex.lock().unwrap();
            // Give the rest of the node time to progress without holding database lock
            lock = self
                .condition
                .wait_timeout(
                    lock,
                    Duration::from_millis(1000 / self.config.frequency as u64),
                )
                .unwrap()
                .0;
        }
    }

    fn activate(&self, txn: &dyn Transaction, account: &Account, account_info: &AccountInfo) {
        let conf_info = self
            .ledger
            .store
            .confirmation_height
            .get(txn, account)
            .unwrap_or_default();

        // If conf info is empty then it means nothing is confirmed yet
        if conf_info.height < account_info.block_count {
            self.stats.inc(StatType::Backlog, DetailType::Activated);

            let callback_lock = self.activate_callback.lock().unwrap();
            if let Some(callback) = &*callback_lock {
                callback(txn, account);
            }

            self.election_schedulers
                .activate_backlog(txn, account, account_info, &conf_info);
        }
    }
}
