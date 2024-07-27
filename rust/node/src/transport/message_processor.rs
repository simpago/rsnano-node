use super::{ChannelEnum, InboundMessageQueue, Origin, RealtimeMessageHandler};
use crate::config::{NodeConfig, NodeFlags};
use rsnano_core::{utils::TomlWriter, NoValue};
use rsnano_messages::DeserializedMessage;
use std::{
    cmp::{max, min},
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
    time::Instant,
};
use tracing::debug;

#[derive(Clone)]
pub struct MessageProcessorConfig {
    pub threads: usize,
    pub max_queue: usize,
}

impl MessageProcessorConfig {
    pub fn new(parallelism: usize) -> Self {
        Self {
            threads: min(2, max(parallelism / 4, 1)),
            max_queue: 64,
        }
    }
}

impl MessageProcessorConfig {
    pub fn serialize_toml(&self, toml: &mut dyn TomlWriter) -> anyhow::Result<()> {
        toml.put_usize(
            "threads",
            self.threads,
            "Number of threads to use for message processing. \ntype:uint64",
        )?;

        toml.put_usize(
            "max_queue",
            self.max_queue,
            "Maximum number of messages per peer to queue for processing. \ntype:uint64",
        )
    }
}

/// Process inbound messages from other nodes
pub struct MessageProcessor {
    flags: NodeFlags,
    config: NodeConfig,
    processing_threads: Vec<JoinHandle<()>>,
    state: Arc<State>,
}

impl MessageProcessor {
    pub fn new(
        flags: NodeFlags,
        config: NodeConfig,
        inbound_queue: Arc<InboundMessageQueue>,
        realtime_handler: Arc<RealtimeMessageHandler>,
    ) -> Self {
        Self {
            flags,
            config,
            processing_threads: Vec::new(),
            state: Arc::new(State {
                inbound_queue,
                realtime_handler,
                stopped: AtomicBool::new(false),
            }),
        }
    }

    pub fn start(&mut self) {
        if !self.flags.disable_tcp_realtime {
            for _ in 0..self.config.network_threads {
                let state = self.state.clone();
                self.processing_threads.push(
                    std::thread::Builder::new()
                        .name("Msg processing".to_string())
                        .spawn(move || {
                            state.run();
                        })
                        .unwrap(),
                );
            }
        }
    }

    pub fn stop(&mut self) {
        self.state.stopped.store(true, Ordering::SeqCst);
        self.state.inbound_queue.stop();
        for t in self.processing_threads.drain(..) {
            t.join().unwrap();
        }
    }
}

impl Drop for MessageProcessor {
    fn drop(&mut self) {
        // All threads must be stopped before this destructor
        debug_assert!(self.processing_threads.is_empty());
    }
}

struct State {
    stopped: AtomicBool,
    realtime_handler: Arc<RealtimeMessageHandler>,
    inbound_queue: Arc<InboundMessageQueue>,
}

impl State {
    const MAX_BATCH_SIZE: usize = 1024 * 4;

    fn run(&self) {
        while !self.stopped.load(Ordering::SeqCst) {
            let batch = self.inbound_queue.next_batch(Self::MAX_BATCH_SIZE);
            if !batch.is_empty() {
                self.handle_batch(batch);
            } else {
                self.inbound_queue.wait_for_messages();
            }
        }
    }

    fn handle_batch(
        &self,
        batch: VecDeque<((DeserializedMessage, Arc<ChannelEnum>), Origin<NoValue>)>,
    ) {
        let start = Instant::now();
        let batch_size = batch.len();
        for ((message, channel), _) in batch {
            self.realtime_handler.process(message.message, &channel);
        }

        let elapsed_millis = start.elapsed().as_millis();
        if elapsed_millis > 100 {
            debug!(
                "Processed {} messages in {} milliseconds (rate of {} messages per second)",
                batch_size,
                elapsed_millis,
                batch_size as u128 * 1000 / elapsed_millis
            );
        }
    }
}
