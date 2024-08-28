use crate::{
    config::{NodeConfig, NodeFlags},
    utils::ThreadPool,
};
use rsnano_core::{Account, BlockHash};
use rsnano_ledger::{Ledger, Writer};
use rsnano_store_lmdb::Transaction;
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tracing::debug;

pub struct LedgerPruning {
    config: NodeConfig,
    flags: NodeFlags,
    ledger: Arc<Ledger>,
    stopped: AtomicBool,
    workers: Arc<dyn ThreadPool>,
}

impl LedgerPruning {
    pub fn new(
        config: NodeConfig,
        flags: NodeFlags,
        ledger: Arc<Ledger>,
        workers: Arc<dyn ThreadPool>,
    ) -> Self {
        Self {
            config,
            flags,
            ledger,
            workers,
            stopped: AtomicBool::new(false),
        }
    }

    pub fn stop(&self) {
        self.stopped.store(true, Ordering::SeqCst);
    }

    pub fn ledger_pruning(&self, batch_size_a: u64, bootstrap_weight_reached_a: bool) {
        let max_depth = if self.config.max_pruning_depth != 0 {
            self.config.max_pruning_depth
        } else {
            u64::MAX
        };
        let cutoff_time = if bootstrap_weight_reached_a {
            (SystemTime::now() - Duration::from_secs(self.config.max_pruning_age_s as u64))
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
        } else {
            u64::MAX
        };
        let mut pruned_count = 0;
        let mut transaction_write_count = 0;
        let mut last_account = Account::from(1); // 0 Burn account is never opened. So it can be used to break loop
        let mut pruning_targets = VecDeque::new();
        let mut target_finished = false;
        while (transaction_write_count != 0 || !target_finished)
            && !self.stopped.load(Ordering::SeqCst)
        {
            // Search pruning targets
            while pruning_targets.len() < batch_size_a as usize
                && !target_finished
                && !self.stopped.load(Ordering::SeqCst)
            {
                target_finished = self.collect_ledger_pruning_targets(
                    &mut pruning_targets,
                    &mut last_account,
                    batch_size_a * 2,
                    max_depth,
                    cutoff_time,
                );
            }
            // Pruning write operation
            transaction_write_count = 0;
            if !pruning_targets.is_empty() && !self.stopped.load(Ordering::SeqCst) {
                let _scoped_write_guard = self.ledger.write_queue.wait(Writer::Pruning);
                let mut tx = self.ledger.rw_txn();
                while !pruning_targets.is_empty()
                    && transaction_write_count < batch_size_a
                    && !self.stopped.load(Ordering::SeqCst)
                {
                    let pruning_hash = pruning_targets.front().unwrap();
                    let account_pruned_count =
                        self.ledger
                            .pruning_action(&mut tx, pruning_hash, batch_size_a);
                    transaction_write_count += account_pruned_count;
                    pruning_targets.pop_front();
                }
                pruned_count += transaction_write_count;

                debug!("Pruned blocks: {}", pruned_count);
            }
        }

        debug!("Total recently pruned block count: {}", pruned_count);
    }

    pub fn collect_ledger_pruning_targets(
        &self,
        pruning_targets_a: &mut VecDeque<BlockHash>,
        last_account_a: &mut Account,
        batch_read_size_a: u64,
        max_depth_a: u64,
        cutoff_time_a: u64,
    ) -> bool {
        let mut read_operations = 0;
        let mut finish_transaction = false;
        let mut tx = self.ledger.read_txn();
        let mut it = self
            .ledger
            .store
            .confirmation_height
            .begin_at_account(&tx, &last_account_a);
        while let Some((&account, info)) = it.current() {
            if finish_transaction {
                break;
            }
            read_operations += 1;
            let mut hash = info.frontier;
            let mut depth = 0;
            while !hash.is_zero() && depth < max_depth_a {
                if let Some(block) = self.ledger.any().get_block(&tx, &hash) {
                    if block.sideband().unwrap().timestamp > cutoff_time_a || depth == 0 {
                        hash = block.previous();
                    } else {
                        break;
                    }
                } else {
                    assert!(depth != 0);
                    hash = BlockHash::zero();
                }
                depth += 1;
                if depth % batch_read_size_a == 0 {
                    drop(it);
                    tx.refresh();
                    it = self
                        .ledger
                        .store
                        .confirmation_height
                        .begin_at_account(&tx, &account);
                }
            }
            if !hash.is_zero() {
                pruning_targets_a.push_back(hash);
            }
            read_operations += depth;
            if read_operations >= batch_read_size_a {
                *last_account_a = account.inc().unwrap_or_default();
                finish_transaction = true;
            } else {
                it.next();
            }
        }

        !finish_transaction || last_account_a.is_zero()
    }
}

pub trait LedgerPruningExt {
    fn start(&self);
    fn ongoing_ledger_pruning(&self);
}

impl LedgerPruningExt for Arc<LedgerPruning> {
    fn start(&self) {
        let self_w = Arc::downgrade(self);
        self.workers.push_task(Box::new(move || {
            if let Some(self_l) = self_w.upgrade() {
                self_l.ongoing_ledger_pruning();
            }
        }));
    }

    fn ongoing_ledger_pruning(&self) {
        let bootstrap_weight_reached =
            self.ledger.block_count() >= self.ledger.bootstrap_weight_max_blocks();
        self.ledger_pruning(
            if self.flags.block_processor_batch_size != 0 {
                self.flags.block_processor_batch_size as u64
            } else {
                2 * 1024
            },
            bootstrap_weight_reached,
        );
        let ledger_pruning_interval = if bootstrap_weight_reached {
            Duration::from_secs(self.config.max_pruning_age_s as u64)
        } else {
            Duration::from_secs(std::cmp::min(self.config.max_pruning_age_s as u64, 15 * 60))
        };
        let node_w = Arc::downgrade(self);
        self.workers.add_delayed_task(
            ledger_pruning_interval,
            Box::new(move || {
                if let Some(node) = node_w.upgrade() {
                    node.ongoing_ledger_pruning()
                }
            }),
        );
    }
}