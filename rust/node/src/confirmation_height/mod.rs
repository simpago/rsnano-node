use rsnano_core::{utils::Logger, Account, BlockEnum, BlockHash, ConfirmationHeightInfo};
use rsnano_ledger::{Ledger, WriteGuard};
use rsnano_store_traits::{Table, Transaction};
use std::{
    collections::{HashMap, VecDeque},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, Weak,
    },
    time::{Duration, Instant},
};

use crate::{
    config::Logging,
    stats::{DetailType, Direction, Stat, StatType},
};

pub struct ConfirmationHeightUnbounded {
    ledger: Arc<Ledger>,
    logger: Arc<dyn Logger>,
    logging: Logging,
    stats: Arc<Stat>,
    pub pending_writes: VecDeque<ConfHeightDetails>,
    pub confirmed_iterated_pairs: HashMap<Account, ConfirmedIteratedPair>,

    //todo: Remove Mutex
    pub implicit_receive_cemented_mapping: HashMap<BlockHash, Weak<Mutex<ConfHeightDetails>>>,
    pub block_cache: Mutex<HashMap<BlockHash, Arc<BlockEnum>>>,

    // All of the atomic variables here just track the size for use in collect_container_info.
    // This is so that no mutexes are needed during the algorithm itself, which would otherwise be needed
    // for the sake of a rarely used RPC call for debugging purposes. As such the sizes are not being acted
    // upon in any way (does not synchronize with any other data).
    // This allows the load and stores to use relaxed atomic memory ordering.
    pub confirmed_iterated_pairs_size: AtomicUsize,
    pub pending_writes_size: AtomicUsize,
    pub implicit_receive_cemented_mapping_size: AtomicUsize,
    timer: Instant,
    batch_separate_pending_min_time: Duration,
    notify_observers_callback: Box<dyn Fn(&Vec<Arc<BlockEnum>>)>,
}

impl ConfirmationHeightUnbounded {
    pub fn new(
        ledger: Arc<Ledger>,
        logger: Arc<dyn Logger>,
        logging: Logging,
        stats: Arc<Stat>,
        batch_separate_pending_min_time: Duration,
        notify_observers_callback: Box<dyn Fn(&Vec<Arc<BlockEnum>>)>,
    ) -> Self {
        Self {
            ledger,
            logger,
            logging,
            stats,
            pending_writes: VecDeque::new(),
            confirmed_iterated_pairs: HashMap::new(),
            implicit_receive_cemented_mapping: HashMap::new(),
            block_cache: Mutex::new(HashMap::new()),
            confirmed_iterated_pairs_size: AtomicUsize::new(0),
            pending_writes_size: AtomicUsize::new(0),
            implicit_receive_cemented_mapping_size: AtomicUsize::new(0),
            timer: Instant::now(),
            batch_separate_pending_min_time,
            notify_observers_callback,
        }
    }

    pub fn pending_empty(&self) -> bool {
        self.pending_writes.is_empty()
    }

    pub fn add_confirmed_iterated_pair(
        &mut self,
        account: Account,
        confirmed_height: u64,
        iterated_height: u64,
    ) {
        self.confirmed_iterated_pairs.insert(
            account,
            ConfirmedIteratedPair {
                confirmed_height,
                iterated_height,
            },
        );
        self.confirmed_iterated_pairs_size
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_pending_write(&mut self, details: ConfHeightDetails) {
        self.pending_writes.push_back(details);
        self.pending_writes_size.fetch_add(1, Ordering::Relaxed);
    }

    pub fn erase_first_pending_write(&mut self) {
        self.pending_writes.pop_front();
        self.pending_writes_size.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn total_pending_write_block_count(&self) -> u64 {
        self.pending_writes
            .iter()
            .map(|x| x.num_blocks_confirmed)
            .sum()
    }

    pub fn add_implicit_receive_cemented(
        &mut self,
        hash: BlockHash,
        details: &Arc<Mutex<ConfHeightDetails>>,
    ) {
        let details = Arc::downgrade(&details);
        self.implicit_receive_cemented_mapping.insert(hash, details);
        self.implicit_receive_cemented_mapping_size.store(
            self.implicit_receive_cemented_mapping.len(),
            Ordering::Relaxed,
        );
    }

    pub fn get_implicit_receive_cemented(
        &self,
        hash: &BlockHash,
    ) -> Option<&Weak<Mutex<ConfHeightDetails>>> {
        self.implicit_receive_cemented_mapping.get(hash)
    }

    pub fn cache_block(&self, block: Arc<BlockEnum>) {
        self.block_cache.lock().unwrap().insert(block.hash(), block);
    }

    pub fn get_blocks(&self, details: &ConfHeightDetails) -> Vec<Arc<BlockEnum>> {
        let cache = self.block_cache.lock().unwrap();
        details
            .block_callback_data
            .iter()
            .map(|hash| Arc::clone(cache.get(hash).unwrap()))
            .collect()
    }

    pub fn get_block_and_sideband(
        &self,
        hash: &BlockHash,
        txn: &dyn Transaction,
    ) -> Arc<BlockEnum> {
        let mut cache = self.block_cache.lock().unwrap();
        match cache.get(hash) {
            Some(block) => Arc::clone(block),
            None => {
                let block = self.ledger.get_block(txn, hash).unwrap(); //todo: remove unwrap
                let block = Arc::new(block);
                cache.insert(*hash, Arc::clone(&block));
                block
            }
        }
    }

    pub fn has_iterated_over_block(&self, hash: &BlockHash) -> bool {
        self.block_cache.lock().unwrap().contains_key(hash)
    }

    pub fn block_cache_size(&self) -> usize {
        self.block_cache.lock().unwrap().len()
    }

    pub fn restart_timer(&mut self) {
        self.timer = Instant::now();
    }

    pub fn min_time_exceeded(&self) -> bool {
        self.timer.elapsed() >= self.batch_separate_pending_min_time
    }

    pub fn clear_process_vars(&mut self) {
        // Separate blocks which are pending confirmation height can be batched by a minimum processing time (to improve lmdb disk write performance),
        // so make sure the slate is clean when a new batch is starting.
        self.confirmed_iterated_pairs.clear();
        self.confirmed_iterated_pairs_size
            .store(0, Ordering::Relaxed);

        self.implicit_receive_cemented_mapping.clear();
        self.implicit_receive_cemented_mapping_size
            .store(0, Ordering::Relaxed);

        self.block_cache.lock().unwrap().clear();
    }

    pub fn prepare_iterated_blocks_for_cementing(
        &mut self,
        preparation_data_a: &mut PreparationData,
    ) {
        let receive_details = &preparation_data_a.receive_details;
        let block_height = preparation_data_a.block_height;
        if block_height > preparation_data_a.confirmation_height {
            // Check whether the previous block has been seen. If so, the rest of sends below have already been seen so don't count them
            if let Some((account, _)) = &preparation_data_a.account_it {
                let pair = self.confirmed_iterated_pairs.get_mut(account).unwrap();
                pair.confirmed_height = block_height;
                if block_height > preparation_data_a.iterated_height {
                    pair.iterated_height = block_height;
                }
            } else {
                self.add_confirmed_iterated_pair(
                    preparation_data_a.account,
                    block_height,
                    block_height,
                );
            }

            let num_blocks_confirmed = block_height - preparation_data_a.confirmation_height;
            let block_callback_data = preparation_data_a.block_callback_data.clone();
            if block_callback_data.is_empty() {
                // if receive_details.is_null() {
                // 	block_callback_data = preparation_data_a.orig_block_callback_data;
                // }
                // 		else
                // 		{
                // 			if (preparation_data_a.already_traversed && receive_details.get_source_block_callback_data ().empty ())
                // 			{
                // 				// We are confirming a block which has already been traversed and found no associated receive details for it.
                // 				conf_height_details_weak_ptr above_receive_details_w{ rsnano::rsn_conf_height_unbounded_get_implicit_receive_cemented (handle, preparation_data_a.current.bytes.data ()) };
                // 				debug_assert (!above_receive_details_w.expired ());
                // 				auto above_receive_details = above_receive_details_w.upgrade ();

                // 				auto num_blocks_already_confirmed = above_receive_details.get_num_blocks_confirmed () - (above_receive_details.get_height () - preparation_data_a.confirmation_height);

                // 				auto block_data{ above_receive_details.get_block_callback_data () };
                // 				auto end = block_data.size () - (num_blocks_already_confirmed);
                // 				auto start = end - num_blocks_confirmed;

                // 				block_callback_data.assign (block_data, start, end);
                // 			}
                // 			else
                // 			{
                // 				block_callback_data = receive_details.get_source_block_callback_data ();
                // 			}

                // 			auto num_to_remove = block_callback_data.size () - num_blocks_confirmed;
                // 			block_callback_data.truncate (block_callback_data.size () - num_to_remove);
                // 			receive_details.set_source_block_callback_data (nano::block_hash_vec{});
                // 		}
            }

            // 	nano::confirmation_height_unbounded::conf_height_details details{ preparation_data_a.account, preparation_data_a.current, block_height, num_blocks_confirmed, block_callback_data };
            // 	rsnano::rsn_conf_height_unbounded_pending_writes_add (handle, details.handle);
        }
    }

    pub fn cement_blocks(&mut self, scoped_write_guard_a: &mut WriteGuard) {
        let cemented_batch_timer: Instant;
        let mut cemented_blocks: Vec<Arc<BlockEnum>> = Vec::new();
        let mut error = false;
        {
            let mut transaction = self
                .ledger
                .store
                .tx_begin_write_for(&[Table::ConfirmationHeight])
                .unwrap();
            cemented_batch_timer = Instant::now();
            while !self.pending_writes.is_empty() {
                let mut pending = self.pending_writes.front().unwrap().clone(); //todo: remove unwrap

                let confirmation_height_info = self
                    .ledger
                    .store
                    .confirmation_height()
                    .get(transaction.txn(), &pending.account)
                    .unwrap_or_default();
                let mut confirmation_height = confirmation_height_info.height;

                if pending.height > confirmation_height {
                    let block = self
                        .ledger
                        .store
                        .block()
                        .get(transaction.txn(), &pending.hash);

                    debug_assert!(self.ledger.pruning_enabled() || block.is_some());
                    debug_assert!(
                        self.ledger.pruning_enabled()
                            || block.as_ref().unwrap().sideband().unwrap().height == pending.height
                    );

                    if block.is_none() {
                        if self.ledger.pruning_enabled()
                            && self
                                .ledger
                                .store
                                .pruned()
                                .exists(transaction.txn(), &pending.hash)
                        {
                            self.erase_first_pending_write();
                            continue;
                        } else {
                            let error_str = format!("Failed to write confirmation height for block {} (unbounded processor)", pending.hash);
                            self.logger.always_log(&error_str);
                            eprintln!("{}", error_str);
                            error = true;
                            break;
                        }
                    }
                    let _ = self.stats.add(
                        StatType::ConfirmationHeight,
                        DetailType::BlocksConfirmed,
                        Direction::In,
                        pending.height - confirmation_height,
                        false,
                    );
                    let _ = self.stats.add(
                        StatType::ConfirmationHeight,
                        DetailType::BlocksConfirmedUnbounded,
                        Direction::In,
                        pending.height - confirmation_height,
                        false,
                    );

                    debug_assert!(
                        pending.num_blocks_confirmed == pending.height - confirmation_height
                    );
                    confirmation_height = pending.height;
                    self.ledger
                        .cache
                        .cemented_count
                        .fetch_add(pending.num_blocks_confirmed, Ordering::SeqCst);

                    self.ledger.store.confirmation_height().put(
                        transaction.as_mut(),
                        &pending.account,
                        &ConfirmationHeightInfo::new(confirmation_height, pending.hash),
                    );

                    // Reverse it so that the callbacks start from the lowest newly cemented block and move upwards
                    pending.block_callback_data.reverse();

                    cemented_blocks.append(&mut self.get_blocks(&pending));
                }
                self.erase_first_pending_write();
            }
        }

        let time_spent_cementing = cemented_batch_timer.elapsed();
        if self.logging.timing_logging_value && time_spent_cementing > Duration::from_millis(50) {
            self.logger.always_log(&format!(
                "Cemented {} blocks in {} ms (unbounded processor)",
                cemented_blocks.len(),
                time_spent_cementing.as_millis()
            ));
        }

        scoped_write_guard_a.release();
        (self.notify_observers_callback)(&cemented_blocks);
        assert!(!error);

        debug_assert!(self.pending_writes.len() == 0);
        debug_assert!(self.pending_writes_size.load(Ordering::Relaxed) == 0);
        self.restart_timer();
    }
}

#[derive(Clone)]
pub struct ConfHeightDetails {
    pub account: Account,
    pub hash: BlockHash,
    pub height: u64,
    pub num_blocks_confirmed: u64,
    pub block_callback_data: Vec<BlockHash>,
    pub source_block_callback_data: Vec<BlockHash>,
}

pub struct ConfirmedIteratedPair {
    pub confirmed_height: u64,
    pub iterated_height: u64,
}

#[derive(Clone)]
pub struct ReceiveSourcePair {
    pub receive_details: Arc<Mutex<ConfHeightDetails>>,
    pub source_hash: BlockHash,
}

impl ReceiveSourcePair {
    pub fn new(receive_details: Arc<Mutex<ConfHeightDetails>>, source_hash: BlockHash) -> Self {
        Self {
            receive_details,
            source_hash,
        }
    }
}

pub struct PreparationData<'a> {
    pub block_height: u64,
    pub confirmation_height: u64,
    pub iterated_height: u64,
    pub account_it: Option<(Account, ConfirmedIteratedPair)>,
    pub account: Account,
    pub receive_details: Option<Arc<Mutex<ConfHeightDetails>>>,
    pub already_traversed: bool,
    pub current: BlockHash,
    pub block_callback_data: &'a mut Vec<BlockHash>,
    pub orig_block_callback_data: &'a mut Vec<BlockHash>,
}
