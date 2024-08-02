use super::{BootstrapAttemptLegacy, BootstrapClient};
use crate::transport::{BufferDropPolicy, TrafficType};
use rsnano_core::{utils::MemoryStream, BlockEnum, BlockHash, BlockType};
use rsnano_ledger::Ledger;
use rsnano_messages::Message;
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
}

struct BulkPushClientData {
    current_target: (BlockHash, BlockHash),
    result: Option<bool>,
}

impl BulkPushClient {
    pub fn new(connection: Arc<BootstrapClient>, ledger: Arc<Ledger>) -> Self {
        Self {
            attempt: None,
            connection,
            data: Mutex::new(BulkPushClientData {
                current_target: (BlockHash::zero(), BlockHash::zero()),
                result: None,
            }),
            condition: Condvar::new(),
            ledger,
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
    fn push_block(&self, block: &BlockEnum);
}

impl BulkPushClientExt for Arc<BulkPushClient> {
    fn start(&self) {
        let Some(_attempt) = self.attempt.as_ref().unwrap().upgrade() else {
            return;
        };

        let message = Message::BulkPush;
        let this_l = Arc::clone(self);
        self.connection.send_obsolete(
            &message,
            Some(Box::new(move |ec, _size| {
                if ec.is_ok() {
                    this_l.push();
                } else {
                    debug!("Unable to send bulk push request: {:?}", ec);
                    this_l.set_result(true);
                }
            })),
            BufferDropPolicy::NoLimiterDrop,
            TrafficType::Generic,
        );
    }

    fn send_finished(&self) {
        let this_l = Arc::clone(self);
        let buffer = Arc::new(vec![BlockType::NotABlock as u8]);
        self.connection.send_buffer(
            &buffer,
            Some(Box::new(move |_ec, _size| {
                this_l.set_result(false);
            })),
            BufferDropPolicy::Limiter,
            TrafficType::Generic,
        )
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

    fn push_block(&self, block: &BlockEnum) {
        let mut stream = MemoryStream::new();
        block.serialize(&mut stream);
        let buffer = Arc::new(stream.to_vec());
        let this_w = Arc::downgrade(self);
        self.connection.send_buffer(
            &buffer,
            Some(Box::new(move |ec, _size| {
                let Some(this_l) = this_w.upgrade() else {
                    return;
                };
                if ec.is_ok() {
                    this_l.push();
                } else {
                    debug!("Error sending block during bulk push: {:?}", ec);
                }
            })),
            BufferDropPolicy::Limiter,
            TrafficType::Generic,
        )
    }
}
