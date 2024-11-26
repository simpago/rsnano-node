use super::election_schedulers::ElectionSchedulers;
use crate::block_processing::BlockProcessor;
use rsnano_core::Block;
use rsnano_ledger::{BlockStatus, Ledger};
use rsnano_store_lmdb::LmdbReadTransaction;
use std::sync::{Arc, Mutex};

/// Observes confirmed blocks and dispatches the process_live function.
pub struct ProcessLiveDispatcher {
    ledger: Arc<Ledger>,
    election_schedulers: Arc<ElectionSchedulers>,
    new_unconfirmed_block_observer: Mutex<Vec<Box<dyn Fn(&Block) + Send + Sync>>>,
}

impl ProcessLiveDispatcher {
    pub fn new(ledger: Arc<Ledger>, election_schedulers: Arc<ElectionSchedulers>) -> Self {
        Self {
            ledger,
            election_schedulers,
            new_unconfirmed_block_observer: Mutex::new(Vec::new()),
        }
    }

    fn inspect(&self, result: &BlockStatus, block: &Block, tx: &LmdbReadTransaction) {
        if *result == BlockStatus::Progress {
            self.process_live(block, tx);
        }
    }

    fn process_live(&self, block: &Block, tx: &LmdbReadTransaction) {
        // Start collecting quorum on block
        if self.ledger.dependents_confirmed(tx, block) {
            self.election_schedulers.activate(tx, &block.account());
        }

        {
            let callbacks = self.new_unconfirmed_block_observer.lock().unwrap();
            for callback in callbacks.iter() {
                (callback)(&block);
            }
        }
    }

    pub fn add_new_unconfirmed_block_callback(&self, f: Box<dyn Fn(&Block) + Send + Sync>) {
        self.new_unconfirmed_block_observer.lock().unwrap().push(f);
    }
}

pub trait ProcessLiveDispatcherExt {
    fn connect(&self, block_processor: &BlockProcessor);
}

impl ProcessLiveDispatcherExt for Arc<ProcessLiveDispatcher> {
    fn connect(&self, block_processor: &BlockProcessor) {
        let self_w = Arc::downgrade(self);
        block_processor.add_batch_processed_observer(Box::new(move |batch| {
            if let Some(self_l) = self_w.upgrade() {
                let tx = self_l.ledger.read_txn();
                for (result, context) in batch {
                    let block = context.block.lock().unwrap().clone();
                    self_l.inspect(result, &block, &tx);
                }
            }
        }));
    }
}
