use std::{
    collections::HashMap,
    sync::{atomic::Ordering, Arc, Mutex},
    time::{Duration, Instant},
};

use rsnano_core::{utils::Logger, Account, BlockEnum, BlockHash, ConfirmationHeightInfo};
use rsnano_ledger::{Ledger, WriteDatabaseQueue, Writer};
use rsnano_store_traits::{Table, Transaction, WriteTransaction};

use crate::{
    config::Logging,
    stats::{DetailType, Direction, Stat, StatType},
};

use super::{cement_queue::CementQueue, ConfHeightDetails};

// Cements blocks. That means it increases the confirmation_height of the account
pub(crate) struct BlockCementor {
    batch_start: Instant,
    last_cementation: Instant,
    batch_separate_pending_min_time: Duration,
    write_database_queue: Arc<WriteDatabaseQueue>,
    ledger: Arc<Ledger>,
    logger: Arc<dyn Logger>,
    logging: Logging,
    stats: Arc<Stat>,
    notify_observers_callback: Box<dyn Fn(&Vec<Arc<BlockEnum>>)>,
    block_cache: Mutex<HashMap<BlockHash, Arc<BlockEnum>>>,
}

impl BlockCementor {
    pub(crate) fn new(
        batch_separate_pending_min_time: Duration,
        write_database_queue: Arc<WriteDatabaseQueue>,
        ledger: Arc<Ledger>,
        logger: Arc<dyn Logger>,
        logging: Logging,
        stats: Arc<Stat>,
        notify_observers_callback: Box<dyn Fn(&Vec<Arc<BlockEnum>>)>,
    ) -> Self {
        Self {
            last_cementation: Instant::now(),
            batch_start: Instant::now(),
            batch_separate_pending_min_time,
            write_database_queue,
            ledger,
            logger,
            logging,
            stats,
            notify_observers_callback,
            block_cache: Mutex::new(HashMap::new()),
        }
    }

    pub fn set_last_cementation(&mut self) {
        self.last_cementation = Instant::now();
    }

    pub fn min_time_exceeded(&self) -> bool {
        self.last_cementation.elapsed() >= self.batch_separate_pending_min_time
    }

    pub fn cement_blocks(
        &mut self,
        cement_queue: &mut CementQueue,
        block_cache: &HashMap<BlockHash, Arc<BlockEnum>>,
    ) {
        let write_guard = self.write_database_queue.wait(Writer::ConfirmationHeight);
        self.batch_start = Instant::now();
        let mut cemented_blocks = Vec::new();
        let mut txn = self
            .ledger
            .store
            .tx_begin_write_for(&[Table::ConfirmationHeight])
            .unwrap();

        while let Some(pending) = cement_queue.pop() {
            self.process_pending_entry(txn.as_mut(), pending, block_cache, &mut cemented_blocks);
        }
        drop(txn);
        drop(write_guard);

        self.log_cemented_count(&cemented_blocks);
        (self.notify_observers_callback)(&cemented_blocks);
        debug_assert!(cement_queue.len() == 0);
        debug_assert!(cement_queue.atomic_len().load(Ordering::Relaxed) == 0);
        self.set_last_cementation();
    }

    fn log_cemented_count(&self, cemented_blocks: &Vec<Arc<BlockEnum>>) {
        let time_spent_cementing = self.batch_start.elapsed();
        if self.logging.timing_logging_value && time_spent_cementing > Duration::from_millis(50) {
            self.logger.always_log(&format!(
                "Cemented {} blocks in {} ms (unbounded processor)",
                cemented_blocks.len(),
                time_spent_cementing.as_millis()
            ));
        }
    }

    fn process_pending_entry(
        &self,
        txn: &mut dyn WriteTransaction,
        pending: ConfHeightDetails,
        block_cache: &HashMap<BlockHash, Arc<BlockEnum>>,
        cemented_blocks: &mut Vec<Arc<BlockEnum>>,
    ) {
        let old_conf_height = self.get_confirmation_height(txn.txn(), &pending.account);

        if pending.new_height <= old_conf_height {
            return;
        }

        match self.check_block_exists(txn.txn(), &pending.hash) {
            BlockResult::BlockExists => {}
            BlockResult::BlockWasPruned => {}
            BlockResult::BlockNotFound => panic!(
                "Failed to write confirmation height for block {}",
                pending.hash
            ),
        }

        debug_assert!(pending.num_blocks_confirmed == pending.new_height - old_conf_height);

        self.write_confirmation_height(txn, &pending);
        self.notify_num_blocks_confirmed(&pending);

        // Reverse it so that the callbacks start from the lowest newly cemented block and move upwards
        for hash in pending.block_callback_data.iter().rev() {
            cemented_blocks.push(Arc::clone(block_cache.get(hash).unwrap()));
        }
    }

    fn get_confirmation_height(&self, txn: &dyn Transaction, account: &Account) -> u64 {
        self.ledger
            .store
            .confirmation_height()
            .get(txn, account)
            .unwrap_or_default()
            .height
    }

    fn check_block_exists(&self, txn: &dyn Transaction, hash: &BlockHash) -> BlockResult {
        let block = self.ledger.store.block().get(txn, hash);
        match block {
            Some(_) => BlockResult::BlockExists,
            None => {
                if self.ledger.pruning_enabled() && self.ledger.store.pruned().exists(txn, &hash) {
                    BlockResult::BlockWasPruned
                } else {
                    BlockResult::BlockNotFound
                }
            }
        }
    }

    fn write_confirmation_height(
        &self,
        txn: &mut dyn WriteTransaction,
        conf_height: &ConfHeightDetails,
    ) {
        self.ledger.store.confirmation_height().put(
            txn,
            &conf_height.account,
            &ConfirmationHeightInfo::new(conf_height.new_height, conf_height.hash),
        );

        self.ledger
            .cache
            .cemented_count
            .fetch_add(conf_height.num_blocks_confirmed, Ordering::SeqCst);
    }

    fn notify_num_blocks_confirmed(&self, pending: &super::ConfHeightDetails) {
        let _ = self.stats.add(
            StatType::ConfirmationHeight,
            DetailType::BlocksConfirmed,
            Direction::In,
            pending.num_blocks_confirmed,
            false,
        );
        let _ = self.stats.add(
            StatType::ConfirmationHeight,
            DetailType::BlocksConfirmedUnbounded,
            Direction::In,
            pending.num_blocks_confirmed,
            false,
        );
    }
}

enum BlockResult {
    BlockExists,
    BlockWasPruned,
    BlockNotFound,
}
