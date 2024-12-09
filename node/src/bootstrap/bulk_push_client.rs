use super::{BootstrapAttemptLegacy, BootstrapClient};
use crate::utils::ThreadPool;
use rsnano_core::{utils::MemoryStream, Block, BlockHash, BlockType};
use rsnano_ledger::Ledger;
use rsnano_messages::Message;
use rsnano_network::TrafficType;
use std::{
    sync::{Arc, Mutex},
    sync::{Condvar, Weak},
};
use tracing::debug;

pub struct BulkPushClient {
    attempt: Option<Weak<BootstrapAttemptLegacy>>,
    connection: Arc<BootstrapClient>,
    data: Mutex<BulkPushClientData>,
    condition: Condvar,
    ledger: Arc<Ledger>,
    tokio: tokio::runtime::Handle,
    workers: Arc<dyn ThreadPool>,
}

struct BulkPushClientData {
    current_target: (BlockHash, BlockHash),
    result: Option<bool>,
}

impl BulkPushClient {
    pub fn new(
        connection: Arc<BootstrapClient>,
        ledger: Arc<Ledger>,
        tokio: tokio::runtime::Handle,
        workers: Arc<dyn ThreadPool>,
    ) -> Self {
        Self {
            attempt: None,
            connection,
            data: Mutex::new(BulkPushClientData {
                current_target: (BlockHash::zero(), BlockHash::zero()),
                result: None,
            }),
            condition: Condvar::new(),
            ledger,
            tokio,
            workers,
        }
    }

    pub fn set_attempt(&mut self, attempt: &Arc<BootstrapAttemptLegacy>) {
        self.attempt = Some(Arc::downgrade(attempt));
    }

    pub fn get_result(&self) -> bool {
        let guard = self.data.lock().unwrap();
        if let Some(result) = guard.result {
            return result;
        }
        let guard = self
            .condition
            .wait_while(guard, |i| i.result.is_none())
            .unwrap();
        guard.result.unwrap()
    }

    pub fn set_result(&self, failed: bool) {
        {
            let mut guard = self.data.lock().unwrap();
            guard.result = Some(failed);
        }
        self.condition.notify_all();
    }
}

pub trait BulkPushClientExt {
    fn start(&self);
    fn send_finished(&self);
    fn push(&self);
    fn push_block(&self, block: &Block);
}

impl BulkPushClientExt for Arc<BulkPushClient> {
    fn start(&self) {
        let Some(_attempt) = self.attempt.as_ref().unwrap().upgrade() else {
            return;
        };

        let message = Message::BulkPush;
        let this_l = Arc::clone(self);

        self.tokio.spawn(async move {
            match this_l.connection.send(&message).await {
                Ok(()) => {
                    let workers = this_l.workers.clone();
                    workers.post(Box::new(move || {
                        this_l.push();
                    }));
                }
                Err(e) => {
                    debug!("Unable to send bulk push request: {:?}", e);
                    this_l.set_result(true);
                }
            }
        });
    }

    fn send_finished(&self) {
        let this_l = Arc::clone(self);
        let buffer = Arc::new(vec![BlockType::NotABlock as u8]);
        self.tokio.spawn(async move {
            let _ = this_l
                .connection
                .get_channel()
                .send_buffer(&buffer, TrafficType::Bootstrap)
                .await;
            this_l.set_result(false);
        });
    }

    fn push(&self) {
        let Some(attempt) = self.attempt.as_ref().unwrap().upgrade() else {
            return;
        };

        let mut guard = self.data.lock().unwrap();
        let mut block = None;
        let mut finished = false;

        while block.is_none() && !finished {
            if guard.current_target.0.is_zero() || guard.current_target.0 == guard.current_target.1
            {
                match attempt.request_bulk_push_target() {
                    Some(target) => guard.current_target = target,
                    None => finished = true,
                }
            }
            if !finished {
                {
                    let txn = self.ledger.read_txn();
                    block = self.ledger.any().get_block(&txn, &guard.current_target.0);
                }
                if block.is_none() {
                    guard.current_target.0 = BlockHash::zero();
                } else {
                    debug!(
                        "Bulk pushing range: [{}:{}]",
                        guard.current_target.0, guard.current_target.1
                    );
                }
            }
        }

        if finished {
            drop(guard);
            self.send_finished();
        } else {
            if let Some(block) = block {
                guard.current_target.0 = block.previous();
                drop(guard);
                self.push_block(&block);
            }
        }
    }

    fn push_block(&self, block: &Block) {
        let mut stream = MemoryStream::new();
        block.serialize(&mut stream);
        let buffer = Arc::new(stream.to_vec());
        let this_l = Arc::clone(self);
        let tokio = self.tokio.clone();
        tokio.spawn(async move {
            match this_l
                .connection
                .get_channel()
                .send_buffer(&buffer, TrafficType::Bootstrap)
                .await
            {
                Ok(()) => {
                    let workers = this_l.workers.clone();
                    workers.post(Box::new(move || this_l.push()));
                }
                Err(e) => {
                    debug!("Error sending block during bulk push: {:?}", e);
                }
            }
        });
    }
}
