use super::MessageFlooder;
use crate::utils::ThreadPool;
use rsnano_core::Block;
use rsnano_messages::{Message, Publish};
use rsnano_network::TrafficType;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};

pub(crate) struct BlockFlooder {
    pub message_flooder: Arc<Mutex<MessageFlooder>>,
    pub workers: Arc<dyn ThreadPool>,
}

impl BlockFlooder {
    pub fn flood_block_many(
        &self,
        blocks: VecDeque<Block>,
        callback: Box<dyn FnOnce() + Send + Sync>,
        delay: Duration,
    ) {
        flood(
            blocks,
            callback,
            delay,
            self.message_flooder.clone(),
            self.workers.clone(),
        )
    }
}

fn flood(
    mut blocks: VecDeque<Block>,
    callback: Box<dyn FnOnce() + Send + Sync>,
    delay: Duration,
    message_flooder: Arc<Mutex<MessageFlooder>>,
    workers: Arc<dyn ThreadPool>,
) {
    if let Some(block) = blocks.pop_front() {
        let publish = Message::Publish(Publish::new_forward(block));
        message_flooder
            .lock()
            .unwrap()
            .flood(&publish, TrafficType::BlockBroadcastRpc, 1.0);
        if blocks.is_empty() {
            callback()
        } else {
            let flooder_w = Arc::downgrade(&message_flooder);
            let workers_w = Arc::downgrade(&workers);
            workers.post_delayed(
                delay,
                Box::new(move || {
                    let Some(flooder) = flooder_w.upgrade() else {
                        return;
                    };
                    let Some(workers) = workers_w.upgrade() else {
                        return;
                    };
                    flood(blocks, callback, delay, flooder, workers);
                }),
            );
        }
    }
}
