use super::election_schedulers::ElectionSchedulers;
use crate::{
    block_processing::BlockProcessor,
    websocket::{OutgoingMessageEnvelope, Topic, WebsocketListener},
};
use rsnano_core::{
    utils::{PropertyTree, SerdePropertyTree},
    BlockEnum,
};
use rsnano_ledger::{BlockStatus, Ledger};
use rsnano_store_lmdb::LmdbReadTransaction;
use std::sync::Arc;

/// Observes confirmed blocks and dispatches the process_live function.
pub struct ProcessLiveDispatcher {
    ledger: Arc<Ledger>,
    election_schedulers: Arc<ElectionSchedulers>,
    websocket: Option<Arc<WebsocketListener>>,
}

impl ProcessLiveDispatcher {
    pub fn new(
        ledger: Arc<Ledger>,
        election_schedulers: Arc<ElectionSchedulers>,
        websocket: Option<Arc<WebsocketListener>>,
    ) -> Self {
        Self {
            ledger,
            election_schedulers,
            websocket,
        }
    }

    fn inspect(&self, result: &BlockStatus, block: &BlockEnum, tx: &LmdbReadTransaction) {
        if *result == BlockStatus::Progress {
            self.process_live(block, tx);
        }
    }

    fn process_live(&self, block: &BlockEnum, tx: &LmdbReadTransaction) {
        // Start collecting quorum on block
        if self.ledger.dependents_confirmed(tx, block) {
            self.election_schedulers.activate(tx, &block.account());
        }

        if let Some(websocket) = &self.websocket {
            if websocket.any_subscriber(Topic::NewUnconfirmedBlock) {
                websocket.broadcast(&new_block_arrived_message(block));
            }
        }
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
                    self_l.inspect(result, &context.block, &tx);
                }
            }
        }));
    }
}

fn new_block_arrived_message(block: &BlockEnum) -> OutgoingMessageEnvelope {
    let mut json_block = SerdePropertyTree::new();
    block.serialize_json(&mut json_block).unwrap();
    let subtype = block.sideband().unwrap().details.state_subtype();
    json_block.put_string("subtype", subtype).unwrap();
    OutgoingMessageEnvelope::new(Topic::NewUnconfirmedBlock, json_block.value)
}
