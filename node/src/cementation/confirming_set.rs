use crate::{
    consensus::Election,
    stats::{DetailType, StatType, Stats},
    utils::{ThreadPool, ThreadPoolImpl},
};
use rsnano_core::{utils::ContainerInfo, BlockHash, SavedBlock};
use rsnano_ledger::{Ledger, WriteGuard, Writer};
use rsnano_store_lmdb::LmdbWriteTransaction;
use std::{
    collections::{HashSet, VecDeque},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Condvar, Mutex,
    },
    thread::JoinHandle,
    time::Duration,
};
use tracing::debug;

use super::ordered_entries::{Entry, OrderedEntries};

#[derive(Clone, Debug, PartialEq)]
pub struct ConfirmingSetConfig {
    pub batch_size: usize,
    /// Maximum number of dependent blocks to be stored in memory during processing
    pub max_blocks: usize,
    pub max_queued_notifications: usize,
}

impl Default for ConfirmingSetConfig {
    fn default() -> Self {
        Self {
            batch_size: 256,
            max_blocks: 128 * 128,
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
                    set: OrderedEntries::default(),
                    current: HashSet::new(),
                }),
                stopped: AtomicBool::new(false),
                condition: Condvar::new(),
                ledger,
                stats,
                config,
                observers: Arc::new(Mutex::new(Observers::default())),
                notification_workers: ThreadPoolImpl::create(1, "Conf notif"),
            }),
        }
    }

    pub(crate) fn on_batch_cemented(&self, callback: BatchCementedCallback) {
        self.thread
            .observers
            .lock()
            .unwrap()
            .batch_cemented
            .push(callback);
    }

    pub fn on_cemented(&self, callback: BlockCallback) {
        self.thread
            .observers
            .lock()
            .unwrap()
            .cemented
            .push(callback);
    }

    pub fn on_already_cemented(&self, callback: AlreadyCementedCallback) {
        self.thread
            .observers
            .lock()
            .unwrap()
            .already_cemented
            .push(callback);
    }

    /// Adds a block to the set of blocks to be confirmed
    pub fn add(&self, hash: BlockHash) {
        self.add_with_election(hash, None)
    }

    pub fn add_with_election(&self, hash: BlockHash, election: Option<Arc<Election>>) {
        self.thread.add(hash, election);
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
        let handle = self.join_handle.lock().unwrap().take();
        if let Some(handle) = handle {
            handle.join().unwrap();
        }
        self.thread.notification_workers.stop();
    }

    /// Added blocks will remain in this set until after ledger has them marked as confirmed.
    pub fn contains(&self, hash: &BlockHash) -> bool {
        self.thread.contains(hash)
    }

    pub fn len(&self) -> usize {
        self.thread.len()
    }

    pub fn info(&self) -> ConfirmingSetInfo {
        let guard = self.thread.mutex.lock().unwrap();
        ConfirmingSetInfo {
            size: guard.set.len(),
            max_size: self.thread.config.max_blocks,
        }
    }

    pub fn container_info(&self) -> ContainerInfo {
        let guard = self.thread.mutex.lock().unwrap();
        [("set", guard.set.len(), std::mem::size_of::<BlockHash>())].into()
    }
}

#[derive(Default)]
pub struct ConfirmingSetInfo {
    pub size: usize,
    pub max_size: usize,
}

impl Drop for ConfirmingSet {
    fn drop(&mut self) {
        self.stop();
    }
}

struct ConfirmingSetThread {
    mutex: Mutex<ConfirmingSetImpl>,
    stopped: AtomicBool,
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
            let _guard = self.mutex.lock().unwrap();
            self.stopped.store(true, Ordering::SeqCst);
        }
        self.condition.notify_all();
    }

    fn add(&self, hash: BlockHash, election: Option<Arc<Election>>) {
        let added = {
            let mut guard = self.mutex.lock().unwrap();
            guard.set.insert(Entry { hash, election })
        };

        if added {
            self.condition.notify_all();
            self.stats.inc(StatType::ConfirmingSet, DetailType::Insert);
        } else {
            self.stats
                .inc(StatType::ConfirmingSet, DetailType::Duplicate);
        }
    }

    fn contains(&self, hash: &BlockHash) -> bool {
        let guard = self.mutex.lock().unwrap();
        guard.set.contains(hash) || guard.current.contains(hash)
    }

    fn len(&self) -> usize {
        self.mutex.lock().unwrap().set.len()
    }

    fn run(&self) {
        let mut guard = self.mutex.lock().unwrap();
        while !self.stopped.load(Ordering::SeqCst) {
            if !guard.set.is_empty() {
                let batch = guard.next_batch(self.config.batch_size);

                // Keep track of the blocks we're currently cementing, so that the .contains (...) check is accurate
                debug_assert!(guard.current.is_empty());
                for entry in &batch {
                    guard.current.insert(entry.hash);
                }

                drop(guard);

                self.run_batch(batch);
                guard = self.mutex.lock().unwrap();
            } else {
                guard = self
                    .condition
                    .wait_while(guard, |i| {
                        i.set.is_empty() && !self.stopped.load(Ordering::SeqCst)
                    })
                    .unwrap();
            }
        }
    }

    fn notify(&self, cemented: &mut VecDeque<Context>) {
        let mut batch = VecDeque::new();
        std::mem::swap(&mut batch, cemented);

        let mut guard = self.mutex.lock().unwrap();

        // It's possible that ledger cementing happens faster than the notifications can be processed by other components, cooldown here
        while self.notification_workers.num_queued_tasks() >= self.config.max_queued_notifications {
            self.stats
                .inc(StatType::ConfirmingSet, DetailType::Cooldown);
            guard = self
                .condition
                .wait_timeout_while(guard, Duration::from_millis(100), |_| {
                    !self.stopped.load(Ordering::SeqCst)
                })
                .unwrap()
                .0;
            if self.stopped.load(Ordering::Relaxed) {
                return;
            }
        }

        let observers = self.observers.clone();
        let stats = self.stats.clone();
        self.notification_workers.post(Box::new(move || {
            stats.inc(StatType::ConfirmingSet, DetailType::Notify);
            observers.lock().unwrap().notify_batch(batch);
        }));
    }

    /// We might need to issue multiple notifications if the block we're confirming implicitly confirms more
    fn notify_maybe(
        &self,
        mut write_guard: WriteGuard,
        mut tx: LmdbWriteTransaction,
        cemented: &mut VecDeque<Context>,
    ) -> (WriteGuard, LmdbWriteTransaction) {
        if cemented.len() >= self.config.max_blocks {
            self.stats
                .inc(StatType::ConfirmingSet, DetailType::NotifyIntermediate);
            drop(write_guard);
            tx.commit();

            self.notify(cemented);

            write_guard = self.ledger.write_queue.wait(Writer::ConfirmationHeight);
            tx.renew();
        }
        (write_guard, tx)
    }

    fn run_batch(&self, batch: VecDeque<Entry>) {
        let mut cemented = VecDeque::new();
        let mut already_cemented = VecDeque::new();

        {
            let mut write_guard = self.ledger.write_queue.wait(Writer::ConfirmationHeight);
            let mut tx = self.ledger.rw_txn();

            for entry in batch {
                let hash = entry.hash;
                let election = entry.election;
                let mut cemented_count = 0;
                let mut success = false;
                loop {
                    (write_guard, tx) = self.ledger.refresh_if_needed(write_guard, tx);

                    // Cementing deep dependency chains might take a long time, allow for graceful shutdown, ignore notifications
                    if self.stopped.load(Ordering::Relaxed) {
                        return;
                    }

                    // Issue notifications here, so that `cemented` set is not too large before we add more blocks
                    (write_guard, tx) = self.notify_maybe(write_guard, tx, &mut cemented);

                    self.stats
                        .inc(StatType::ConfirmingSet, DetailType::Cementing);

                    // The block might be rolled back before it's fully cemented
                    if !self.ledger.any().block_exists(&tx, &hash) {
                        self.stats
                            .inc(StatType::ConfirmingSet, DetailType::MissingBlock);
                        break;
                    }

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
                        cemented_count += added.len();
                        for block in added {
                            cemented.push_back(Context {
                                block,
                                confirmation_root: hash,
                                election: election.clone(),
                            });
                        }
                    } else {
                        self.stats
                            .inc(StatType::ConfirmingSet, DetailType::AlreadyCemented);
                        already_cemented.push_back(hash);
                    }

                    success = self.ledger.confirmed().block_exists(&tx, &hash);
                    if success {
                        break;
                    }
                }

                if success {
                    self.stats
                        .inc(StatType::ConfirmingSet, DetailType::CementedHash);
                    debug!(
                        "Cemented block: {} (total cemented: {})",
                        hash, cemented_count
                    );
                } else {
                    self.stats
                        .inc(StatType::ConfirmingSet, DetailType::CementingFailed);
                    debug!("Failed to cement block: {}", hash);
                }
            }
        }

        self.notify(&mut cemented);

        {
            let mut guard = self.observers.lock().unwrap();
            for callback in &mut guard.already_cemented {
                callback(&already_cemented)
            }
        }
        self.mutex.lock().unwrap().current.clear();
    }
}

struct ConfirmingSetImpl {
    set: OrderedEntries,
    current: HashSet<BlockHash>,
}

impl ConfirmingSetImpl {
    fn next_batch(&mut self, max_count: usize) -> VecDeque<Entry> {
        let mut results = VecDeque::new();
        // TODO: use extract_if once it is stablized
        while let Some(entry) = self.set.pop_front() {
            if results.len() >= max_count {
                break;
            }
            results.push_back(entry);
        }
        results
    }
}

type BlockCallback = Box<dyn FnMut(&SavedBlock) + Send>;

/// block + confirmation root
type BatchCementedCallback = Box<dyn FnMut(&VecDeque<Context>) + Send>;
type AlreadyCementedCallback = Box<dyn FnMut(&VecDeque<BlockHash>) + Send>;

#[derive(Default)]
struct Observers {
    cemented: Vec<BlockCallback>,
    batch_cemented: Vec<BatchCementedCallback>,
    already_cemented: Vec<AlreadyCementedCallback>,
}

impl Observers {
    fn notify_batch(&mut self, cemented: VecDeque<Context>) {
        for context in &cemented {
            for observer in &mut self.cemented {
                observer(&context.block);
            }
        }

        for observer in &mut self.batch_cemented {
            observer(&cemented);
        }
    }
}

pub(crate) struct Context {
    pub block: SavedBlock,
    pub confirmation_root: BlockHash,
    pub election: Option<Arc<Election>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsnano_core::{ConfirmationHeightInfo, SavedAccountChain};
    use std::time::Duration;

    #[test]
    fn add_exists() {
        let ledger = Arc::new(Ledger::new_null());
        let confirming_set =
            ConfirmingSet::new(Default::default(), ledger, Arc::new(Stats::default()));
        let hash = BlockHash::from(1);
        confirming_set.add(hash);
        assert!(confirming_set.contains(&hash));
    }

    #[test]
    fn process_one() {
        let mut chain = SavedAccountChain::genesis();
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
        confirming_set.on_cemented(Box::new(move |_block| {
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
