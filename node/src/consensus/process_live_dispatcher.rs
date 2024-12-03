use super::election_schedulers::ElectionSchedulers;
use crate::block_processing::BlockProcessor;
use rsnano_core::SavedBlock;
use rsnano_ledger::{BlockStatus, Ledger};
use rsnano_store_lmdb::LmdbReadTransaction;
use std::sync::{Arc, Mutex};

/// Observes confirmed blocks and dispatches the process_live function.
pub struct ProcessLiveDispatcher {
    ledger: Arc<Ledger>,
    election_schedulers: Arc<ElectionSchedulers>,
    new_unconfirmed_block_observer: Mutex<Vec<Arc<dyn Fn(&SavedBlock) + Send + Sync>>>,
}

impl ProcessLiveDispatcher {
    pub fn new(ledger: Arc<Ledger>, election_schedulers: Arc<ElectionSchedulers>) -> Self {
        Self {
            ledger,
            election_schedulers,
            new_unconfirmed_block_observer: Mutex::new(Vec::new()),
        }
    }

    fn process_live(&self, block: &SavedBlock, tx: &LmdbReadTransaction) {
        // Start collecting quorum on block
        if self.ledger.dependents_confirmed(tx, block) {
            self.election_schedulers.activate(tx, &block.account());
        }

        let callbacks = {
            let callbacks_guard = self.new_unconfirmed_block_observer.lock().unwrap();
            callbacks_guard.clone()
        };

        for callback in callbacks.iter() {
            callback(block);
        }
    }

    pub fn add_new_unconfirmed_block_callback(&self, f: Arc<dyn Fn(&SavedBlock) + Send + Sync>) {
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
                    if *result == BlockStatus::Progress {
                        let block = context
                            .saved_block
                            .lock()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .clone();
                        self_l.process_live(&block, &tx);
                    }
                }
            }
        }));
    }
}
