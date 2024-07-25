use super::{BatchCementedCallback, BlockCallback};
use crate::{
    stats::{DetailType, StatType, Stats},
    utils::{ThreadPool, ThreadPoolImpl},
};
use rsnano_core::{
    utils::{ContainerInfo, ContainerInfoComponent},
    BlockEnum, BlockHash,
};
use rsnano_ledger::{Ledger, WriteGuard, Writer};
use rsnano_store_lmdb::LmdbWriteTransaction;
use std::{
    collections::{HashSet, VecDeque},
    sync::{Arc, Condvar, Mutex},
    thread::{sleep, JoinHandle},
    time::Duration,
};

#[derive(Clone)]
pub struct ConfirmingSetConfig {
    /// Maximum number of dependent blocks to be stored in memory during processing
    pub max_blocks: usize,
    pub max_queued_notifications: usize,
}

impl Default for ConfirmingSetConfig {
    fn default() -> Self {
        Self {
            max_blocks: 64 * 128,
            max_queued_notifications: 8,
        }
    }
}

/// Set of blocks to be durably confirmed
pub struct ConfirmingSet {
    thread: Arc<ConfirmingSetThread>,
    join_handle: Mutex<Option<JoinHandle<()>>>,
}

impl ConfirmingSet {
    pub fn new(config: ConfirmingSetConfig, ledger: Arc<Ledger>, stats: Arc<Stats>) -> Self {
        Self {
            join_handle: Mutex::new(None),
            thread: Arc::new(ConfirmingSetThread {
                mutex: Mutex::new(ConfirmingSetImpl {
                    stopped: false,
                    set: HashSet::new(),
                }),
                condition: Condvar::new(),
                ledger,
                stats,
                config,
                observers: Arc::new(Mutex::new(Observers::default())),
                notification_workers: ThreadPoolImpl::create(1, "Conf notif"),
            }),
        }
    }

    pub(crate) fn add_batch_cemented_observer(&self, callback: BatchCementedCallback) {
        self.thread
            .observers
            .lock()
            .unwrap()
            .batch_cemented
            .push(callback);
    }

    pub fn add_cemented_observer(&self, callback: BlockCallback) {
        self.thread
            .observers
            .lock()
            .unwrap()
            .cemented
            .push(callback);
    }

    /// Adds a block to the set of blocks to be confirmed
    pub fn add(&self, hash: BlockHash) {
        self.thread.add(hash);
    }

    pub fn start(&self) {
        debug_assert!(self.join_handle.lock().unwrap().is_none());

        let thread = Arc::clone(&self.thread);
        *self.join_handle.lock().unwrap() = Some(
            std::thread::Builder::new()
                .name("Conf height".to_string())
                .spawn(move || thread.run())
                .unwrap(),
        );
    }

    pub fn stop(&self) {
        self.thread.stop();
        if let Some(handle) = self.join_handle.lock().unwrap().take() {
            handle.join().unwrap();
        }
        self.thread.notification_workers.stop();
    }

    /// Added blocks will remain in this set until after ledger has them marked as confirmed.
    pub fn exists(&self, hash: &BlockHash) -> bool {
        self.thread.exists(hash)
    }

    pub fn len(&self) -> usize {
        self.thread.len()
    }

    pub fn collect_container_info(&self, name: impl Into<String>) -> ContainerInfoComponent {
        let guard = self.thread.mutex.lock().unwrap();
        ContainerInfoComponent::Composite(
            name.into(),
            vec![ContainerInfoComponent::Leaf(ContainerInfo {
                name: "set".to_string(),
                count: guard.set.len(),
                sizeof_element: std::mem::size_of::<BlockHash>(),
            })],
        )
    }
}

impl Drop for ConfirmingSet {
    fn drop(&mut self) {
        self.stop();
    }
}

struct ConfirmingSetThread {
    mutex: Mutex<ConfirmingSetImpl>,
    condition: Condvar,
    ledger: Arc<Ledger>,
    stats: Arc<Stats>,
    config: ConfirmingSetConfig,
    notification_workers: ThreadPoolImpl,
    observers: Arc<Mutex<Observers>>,
}

impl ConfirmingSetThread {
    fn stop(&self) {
        {
            let mut guard = self.mutex.lock().unwrap();
            guard.stopped = true;
        }
        self.condition.notify_all();
    }

    fn add(&self, hash: BlockHash) {
        let added = {
            let mut guard = self.mutex.lock().unwrap();
            guard.set.insert(hash)
        };

        if added {
            self.condition.notify_all();
            self.stats.inc(StatType::ConfirmingSet, DetailType::Insert);
        } else {
            self.stats
                .inc(StatType::ConfirmingSet, DetailType::Duplicate);
        }
    }

    fn exists(&self, hash: &BlockHash) -> bool {
        self.mutex.lock().unwrap().set.contains(hash)
    }

    fn len(&self) -> usize {
        self.mutex.lock().unwrap().set.len()
    }

    fn run(&self) {
        let mut guard = self.mutex.lock().unwrap();
        while !guard.stopped {
            if !guard.set.is_empty() {
                let batch = guard.next_batch(256);
                drop(guard);
                self.run_batch(batch);
                guard = self.mutex.lock().unwrap();
            } else {
                guard = self
                    .condition
                    .wait_while(guard, |i| i.set.is_empty() && !i.stopped)
                    .unwrap();
            }
        }
    }

    fn notify(
        &self,
        cemented: &mut VecDeque<(BlockEnum, BlockHash)>,
        already_cemented: &mut VecDeque<BlockHash>,
    ) {
        let mut notification = CementedNotification {
            cemented: VecDeque::new(),
            already_cemented: VecDeque::new(),
        };

        std::mem::swap(&mut notification.cemented, cemented);
        std::mem::swap(&mut notification.already_cemented, already_cemented);

        // Wait for the worker thread if too many notifications are queued
        while self.notification_workers.num_queued_tasks() >= self.config.max_queued_notifications {
            self.stats
                .inc(StatType::ConfirmingSet, DetailType::Cooldown);
            sleep(Duration::from_millis(100));
        }

        let observers = self.observers.clone();
        let stats = self.stats.clone();
        self.notification_workers.push_task(Box::new(move || {
            stats.inc(StatType::ConfirmingSet, DetailType::Notify);
            observers.lock().unwrap().notify_batch(notification);
        }));
    }

    /// We might need to issue multiple notifications if the block we're confirming implicitly confirms more
    fn notify_maybe(
        &self,
        mut write_guard: WriteGuard,
        mut tx: LmdbWriteTransaction,
        cemented: &mut VecDeque<(BlockEnum, BlockHash)>,
        already_cemented: &mut VecDeque<BlockHash>,
    ) -> (WriteGuard, LmdbWriteTransaction) {
        if cemented.len() >= self.config.max_blocks {
            self.stats
                .inc(StatType::ConfirmingSet, DetailType::NotifyIntermediate);
            drop(write_guard);
            tx.commit();

            self.notify(cemented, already_cemented);

            write_guard = self.ledger.write_queue.wait(Writer::ConfirmationHeight);
            tx.renew();
        }
        (write_guard, tx)
    }

    fn run_batch(&self, batch: VecDeque<BlockHash>) {
        let mut cemented = VecDeque::new();
        let mut already_cemented = VecDeque::new();

        {
            let mut write_guard = self.ledger.write_queue.wait(Writer::ConfirmationHeight);
            let mut tx = self.ledger.rw_txn();

            for hash in batch {
                loop {
                    (write_guard, tx) = self.ledger.refresh_if_needed(write_guard, tx);
                    self.stats
                        .inc(StatType::ConfirmingSet, DetailType::CementingHash);

                    // Issue notifications here, so that `cemented` set is not too large before we add more blocks
                    (write_guard, tx) =
                        self.notify_maybe(write_guard, tx, &mut cemented, &mut already_cemented);

                    let added = self
                        .ledger
                        .confirm_max(&mut tx, hash, self.config.max_blocks);
                    let added_len = added.len();
                    if !added.is_empty() {
                        // Confirming this block may implicitly confirm more
                        self.stats.add(
                            StatType::ConfirmingSet,
                            DetailType::Cemented,
                            added_len as u64,
                        );
                        for block in added {
                            cemented.push_back((block, hash));
                        }
                    } else {
                        self.stats
                            .inc(StatType::ConfirmingSet, DetailType::AlreadyCemented);
                        already_cemented.push_back(hash);
                    }

                    if self.ledger.confirmed().block_exists(&tx, &hash)
                        || self.mutex.lock().unwrap().stopped
                    {
                        break;
                    }
                }
            }
        }

        self.notify(&mut cemented, &mut already_cemented);
    }
}

struct ConfirmingSetImpl {
    stopped: bool,
    set: HashSet<BlockHash>,
}

impl ConfirmingSetImpl {
    fn next_batch(&mut self, max_count: usize) -> VecDeque<BlockHash> {
        let mut results = VecDeque::new();
        // TODO: use extract_if once it is stablized
        while let Some(&hash) = self.set.iter().next() {
            if results.len() >= max_count {
                break;
            }
            results.push_back(hash);
            self.set.remove(&hash);
        }
        results
    }
}

pub(crate) struct CementedNotification {
    pub cemented: VecDeque<(BlockEnum, BlockHash)>, // block + confirmation root
    pub already_cemented: VecDeque<BlockHash>,
}

#[derive(Default)]
struct Observers {
    cemented: Vec<BlockCallback>,
    batch_cemented: Vec<BatchCementedCallback>,
}

impl Observers {
    fn notify_batch(&mut self, notification: CementedNotification) {
        for (block, _) in &notification.cemented {
            for observer in &mut self.cemented {
                observer(&Arc::new(block.clone()));
            }
        }

        for observer in &mut self.batch_cemented {
            observer(&notification);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsnano_core::{ConfirmationHeightInfo, TestAccountChain};
    use std::time::Duration;

    #[test]
    fn add_exists() {
        let ledger = Arc::new(Ledger::new_null());
        let confirming_set =
            ConfirmingSet::new(Default::default(), ledger, Arc::new(Stats::default()));
        let hash = BlockHash::from(1);
        confirming_set.add(hash);
        assert!(confirming_set.exists(&hash));
    }

    #[test]
    fn process_one() {
        let mut chain = TestAccountChain::genesis();
        let block_hash = chain.add_state().hash();
        let ledger = Arc::new(
            Ledger::new_null_builder()
                .blocks(chain.blocks())
                .confirmation_height(
                    &chain.account(),
                    &ConfirmationHeightInfo {
                        height: 1,
                        frontier: chain.open(),
                    },
                )
                .finish(),
        );
        let confirming_set =
            ConfirmingSet::new(Default::default(), ledger, Arc::new(Stats::default()));
        confirming_set.start();
        let count = Arc::new(Mutex::new(0));
        let condition = Arc::new(Condvar::new());
        let count_clone = Arc::clone(&count);
        let condition_clone = Arc::clone(&condition);
        confirming_set.add_cemented_observer(Box::new(move |_block| {
            {
                *count_clone.lock().unwrap() += 1;
            }
            condition_clone.notify_all();
        }));

        confirming_set.add(block_hash);

        let guard = count.lock().unwrap();
        let result = condition
            .wait_timeout_while(guard, Duration::from_secs(5), |i| *i < 1)
            .unwrap()
            .1;
        assert_eq!(result.timed_out(), false);
    }
}
