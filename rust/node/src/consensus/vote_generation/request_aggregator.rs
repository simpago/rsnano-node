use super::{
    request_aggregator_impl::{AggregateResult, RequestAggregatorImpl},
    VoteGenerators,
};
use crate::{
    stats::{DetailType, Direction, StatType, Stats},
    transport::{BufferDropPolicy, ChannelEnum, FairQueue, Origin, TrafficType},
};
use rsnano_core::{
    utils::{ContainerInfoComponent, TomlWriter},
    BlockHash, NoValue, Root, Vote,
};
use rsnano_ledger::Ledger;
use rsnano_messages::{ConfirmAck, Message};
use rsnano_store_lmdb::{LmdbReadTransaction, Transaction};
use std::{
    cmp::{max, min},
    sync::{Arc, Condvar, Mutex, MutexGuard},
    thread::JoinHandle,
};
use tracing::trace;

#[derive(Debug, Clone)]
pub struct RequestAggregatorConfig {
    pub threads: usize,
    pub max_queue: usize,
    pub batch_size: usize,
}

impl RequestAggregatorConfig {
    pub fn new(parallelism: usize) -> Self {
        Self {
            threads: max(1, min(parallelism / 2, 4)),
            max_queue: 128,
            batch_size: 16,
        }
    }

    pub fn serialize_toml(&self, toml: &mut dyn TomlWriter) -> anyhow::Result<()> {
        toml.put_usize(
            "max_queue",
            self.max_queue,
            "Maximum number of queued requests per peer. \ntype:uint64",
        )?;
        toml.put_usize(
            "threads",
            self.threads,
            "Number of threads for request processing. \ntype:uint64",
        )?;
        toml.put_usize(
            "batch_size",
            self.batch_size,
            "Number of requests to process in a single batch. \ntype:uint64",
        )
    }
}

/**
 * Pools together confirmation requests, separately for each endpoint.
 * Requests are added from network messages, and aggregated to minimize bandwidth and vote generation. Example:
 * * Two votes are cached, one for hashes {1,2,3} and another for hashes {4,5,6}
 * * A request arrives for hashes {1,4,5}. Another request arrives soon afterwards for hashes {2,3,6}
 * * The aggregator will reply with the two cached votes
 * Votes are generated for uncached hashes.
 */
pub struct RequestAggregator {
    config: RequestAggregatorConfig,
    stats: Arc<Stats>,
    vote_generators: Arc<VoteGenerators>,
    ledger: Arc<Ledger>,
    mutex: Mutex<RequestAggregatorData>,
    condition: Condvar,
    threads: Mutex<Vec<JoinHandle<()>>>,
}

impl RequestAggregator {
    pub fn new(
        config: RequestAggregatorConfig,
        stats: Arc<Stats>,
        vote_generators: Arc<VoteGenerators>,
        ledger: Arc<Ledger>,
    ) -> Self {
        let max_queue = config.max_queue;
        Self {
            stats,
            vote_generators,
            ledger,
            config,
            condition: Condvar::new(),
            mutex: Mutex::new(RequestAggregatorData {
                queue: FairQueue::new(Box::new(move |_| max_queue), Box::new(|_| 1)),
                stopped: false,
            }),
            threads: Mutex::new(Vec::new()),
        }
    }

    pub fn request(&self, request: RequestType, channel: Arc<ChannelEnum>) -> bool {
        // This should be checked before calling request
        debug_assert!(!request.is_empty());
        let request_len = request.len();

        let added = {
            self.mutex
                .lock()
                .unwrap()
                .queue
                .push((request, channel.clone()), Origin::new(NoValue {}, channel))
        };

        if added {
            self.stats
                .inc(StatType::RequestAggregator, DetailType::Request);
            self.stats.add(
                StatType::RequestAggregator,
                DetailType::RequestHashes,
                request_len as u64,
            );
            self.condition.notify_one();
        } else {
            self.stats
                .inc(StatType::RequestAggregator, DetailType::Overfill);
            self.stats.add(
                StatType::RequestAggregator,
                DetailType::OverfillHashes,
                request_len as u64,
            );
        }

        // TODO: This stat is for compatibility with existing tests and is in principle unnecessary
        self.stats.inc(
            StatType::Aggregator,
            if added {
                DetailType::AggregatorAccepted
            } else {
                DetailType::AggregatorDropped
            },
        );

        added
    }

    pub fn run(&self) {
        let mut guard = self.mutex.lock().unwrap();
        while !guard.stopped {
            trace!("loop");

            if !guard.queue.is_empty() {
                guard = self.run_batch(guard);
            } else {
                guard = self
                    .condition
                    .wait_while(guard, |g| !g.stopped && g.queue.is_empty())
                    .unwrap();
            }
        }
    }

    pub fn stop(&self) {
        self.mutex.lock().unwrap().stopped = true;
        self.condition.notify_all();
        let mut threads = Vec::new();
        {
            let mut guard = self.threads.lock().unwrap();
            std::mem::swap(&mut threads, &mut *guard);
        }
        for thread in threads {
            thread.join().unwrap();
        }
    }

    fn run_batch<'a>(
        &'a self,
        mut state: MutexGuard<'a, RequestAggregatorData>,
    ) -> MutexGuard<'a, RequestAggregatorData> {
        let batch = state.queue.next_batch(self.config.batch_size);
        drop(state);

        let mut tx = self.ledger.read_txn();

        for ((request, channel), _) in &batch {
            tx.refresh_if_needed();

            if !channel.max(TrafficType::Generic) {
                self.process(&tx, request, channel);
            } else {
                self.stats.inc_dir(
                    StatType::RequestAggregator,
                    DetailType::ChannelFull,
                    Direction::Out,
                );
            }
        }

        self.mutex.lock().unwrap()
    }

    fn process(&self, tx: &LmdbReadTransaction, request: &RequestType, channel: &Arc<ChannelEnum>) {
        let remaining = self.aggregate(tx, request);

        if !remaining.remaining_normal.is_empty() {
            self.stats
                .inc(StatType::RequestAggregatorReplies, DetailType::NormalVote);

            // Generate votes for the remaining hashes
            let generated = self
                .vote_generators
                .generate_non_final_votes(&remaining.remaining_normal, channel.clone());
            self.stats.add_dir(
                StatType::Requests,
                DetailType::RequestsCannotVote,
                Direction::In,
                (remaining.remaining_normal.len() - generated) as u64,
            );
        }

        if !remaining.remaining_final.is_empty() {
            self.stats
                .inc(StatType::RequestAggregatorReplies, DetailType::FinalVote);

            // Generate final votes for the remaining hashes
            let generated = self
                .vote_generators
                .generate_final_votes(&remaining.remaining_final, channel.clone());
            self.stats.add_dir(
                StatType::Requests,
                DetailType::RequestsCannotVote,
                Direction::In,
                (remaining.remaining_final.len() - generated) as u64,
            );
        }
    }

    /// Returns the number of currently queued request pools
    pub fn len(&self) -> usize {
        self.mutex.lock().unwrap().queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn reply_action(&self, vote: &Arc<Vote>, channel: &ChannelEnum) {
        let confirm = Message::ConfirmAck(ConfirmAck::new_with_own_vote((**vote).clone()));
        channel.send(
            &confirm,
            None,
            BufferDropPolicy::Limiter,
            TrafficType::Generic,
        );
    }

    /// Aggregate requests and send cached votes to channel.
    /// Return the remaining hashes that need vote generation for each block for regular & final vote generators
    fn aggregate(&self, tx: &LmdbReadTransaction, requests: &RequestType) -> AggregateResult {
        let mut aggregator = RequestAggregatorImpl::new(&self.ledger, &self.stats, tx);
        aggregator.add_votes(requests);
        aggregator.get_result()
    }

    pub fn collect_container_info(&self, name: impl Into<String>) -> ContainerInfoComponent {
        let guard = self.mutex.lock().unwrap();
        ContainerInfoComponent::Composite(
            name.into(),
            vec![guard.queue.collect_container_info("queue")],
        )
    }
}

impl Drop for RequestAggregator {
    fn drop(&mut self) {
        debug_assert!(self.threads.lock().unwrap().is_empty())
    }
}

type RequestType = Vec<(BlockHash, Root)>;
type ValueType = (RequestType, Arc<ChannelEnum>);

struct RequestAggregatorData {
    queue: FairQueue<ValueType, NoValue>,
    stopped: bool,
}

pub trait RequestAggregatorExt {
    fn start(&self);
}

impl RequestAggregatorExt for Arc<RequestAggregator> {
    fn start(&self) {
        let self_w = Arc::downgrade(self);
        self.vote_generators
            .set_reply_action(Arc::new(move |vote, channel| {
                if let Some(self_l) = self_w.upgrade() {
                    self_l.reply_action(vote, channel);
                }
            }));

        let mut guard = self.threads.lock().unwrap();
        for _ in 0..self.config.threads {
            let self_l = Arc::clone(self);
            guard.push(
                std::thread::Builder::new()
                    .name("Req aggregator".to_string())
                    .spawn(move || self_l.run())
                    .unwrap(),
            );
        }
    }
}
