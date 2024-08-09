use super::{LocalVoteHistory, VoteSpacing};
use crate::{
    consensus::VoteBroadcaster,
    stats::{DetailType, Direction, StatType, Stats},
    transport::{BufferDropPolicy, Channel, TrafficType},
    utils::ProcessingQueue,
    wallets::Wallets,
};
use rsnano_core::{
    utils::{milliseconds_since_epoch, ContainerInfo, ContainerInfoComponent},
    BlockEnum, BlockHash, Root, Vote,
};
use rsnano_ledger::{Ledger, Writer};
use rsnano_messages::{ConfirmAck, Message};
use rsnano_store_lmdb::{LmdbReadTransaction, LmdbWriteTransaction, Transaction};
use std::{
    collections::VecDeque,
    mem::size_of,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Condvar, Mutex, MutexGuard,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

pub(crate) struct VoteGenerator {
    ledger: Arc<Ledger>,
    vote_generation_queue: ProcessingQueue<(Root, BlockHash)>,
    shared_state: Arc<SharedState>,
    thread: Mutex<Option<JoinHandle<()>>>,
    stats: Arc<Stats>,
}

impl VoteGenerator {
    const MAX_REQUESTS: usize = 2048;
    const MAX_HASHES: usize = 255;

    pub(crate) fn new(
        ledger: Arc<Ledger>,
        wallets: Arc<Wallets>,
        history: Arc<LocalVoteHistory>,
        is_final: bool,
        stats: Arc<Stats>,
        voting_delay: Duration,
        vote_generator_delay: Duration,
        vote_generator_threshold: usize,
        vote_broadcaster: Arc<VoteBroadcaster>,
    ) -> Self {
        let shared_state = Arc::new(SharedState {
            ledger: Arc::clone(&ledger),
            history,
            wallets,
            condition: Condvar::new(),
            queues: Mutex::new(Queues::default()),
            is_final,
            stopped: AtomicBool::new(false),
            stats: Arc::clone(&stats),
            vote_broadcaster,
            spacing: Mutex::new(VoteSpacing::new(voting_delay)),
            vote_generator_delay,
            vote_generator_threshold,
        });

        let shared_state_clone = Arc::clone(&shared_state);
        Self {
            ledger,
            shared_state,
            thread: Mutex::new(None),
            vote_generation_queue: ProcessingQueue::new(
                Arc::clone(&stats),
                StatType::VoteGenerator,
                "Voting que".to_string(),
                1,         // single threaded
                1024 * 32, // max queue size
                256,       // max batch size,
                Box::new(move |batch| {
                    shared_state_clone.process_batch(batch);
                }),
            ),
            stats,
        }
    }

    pub(crate) fn start(&self) {
        let shared_state_clone = Arc::clone(&self.shared_state);
        *self.thread.lock().unwrap() = Some(
            thread::Builder::new()
                .name("voting".to_owned())
                .spawn(move || shared_state_clone.run())
                .unwrap(),
        );
        self.vote_generation_queue.start();
    }

    pub(crate) fn stop(&self) {
        self.vote_generation_queue.stop();
        self.shared_state.stopped.store(true, Ordering::SeqCst);
        self.shared_state.condition.notify_all();
        if let Some(thread) = self.thread.lock().unwrap().take() {
            thread.join().unwrap();
        }
    }

    /// Queue items for vote generation, or broadcast votes already in cache
    pub(crate) fn add(&self, root: &Root, hash: &BlockHash) {
        self.vote_generation_queue.add((*root, *hash));
    }

    /// Queue blocks for vote generation, returning the number of successful candidates.
    pub(crate) fn generate(&self, blocks: &[Arc<BlockEnum>], channel: Arc<Channel>) -> usize {
        let req_candidates = {
            let txn = self.ledger.read_txn();
            blocks
                .iter()
                .filter_map(|i| {
                    if self.ledger.dependents_confirmed(&txn, i) {
                        Some((i.root(), i.hash()))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        };

        let result = req_candidates.len();
        let mut guard = self.shared_state.queues.lock().unwrap();
        guard.requests.push_back((req_candidates, channel));
        while guard.requests.len() > Self::MAX_REQUESTS {
            // On a large queue of requests, erase the oldest one
            guard.requests.pop_front();
            self.stats.inc(
                StatType::VoteGenerator,
                DetailType::GeneratorRepliesDiscarded,
            );
        }

        result
    }

    pub(crate) fn collect_container_info(&self, name: impl Into<String>) -> ContainerInfoComponent {
        let candidates_count;
        let requests_count;
        {
            let guard = self.shared_state.queues.lock().unwrap();
            candidates_count = guard.candidates.len();
            requests_count = guard.requests.len();
        }
        ContainerInfoComponent::Composite(
            name.into(),
            vec![
                ContainerInfoComponent::Leaf(ContainerInfo {
                    name: "candidates".to_string(),
                    count: candidates_count,
                    sizeof_element: size_of::<Root>() + size_of::<BlockHash>(),
                }),
                ContainerInfoComponent::Leaf(ContainerInfo {
                    name: "requests".to_string(),
                    count: requests_count,
                    sizeof_element: size_of::<Arc<Channel>>() + size_of::<Vec<(Root, BlockHash)>>(),
                }),
            ],
        )
    }
}

impl Drop for VoteGenerator {
    fn drop(&mut self) {
        debug_assert!(self.thread.lock().unwrap().is_none())
    }
}

struct SharedState {
    ledger: Arc<Ledger>,
    wallets: Arc<Wallets>,
    history: Arc<LocalVoteHistory>,
    is_final: bool,
    condition: Condvar,
    stopped: AtomicBool,
    queues: Mutex<Queues>,
    stats: Arc<Stats>,
    vote_broadcaster: Arc<VoteBroadcaster>,
    spacing: Mutex<VoteSpacing>,
    vote_generator_delay: Duration,
    vote_generator_threshold: usize,
}

impl SharedState {
    fn run(&self) {
        let mut queues = self.queues.lock().unwrap();
        while !self.stopped.load(Ordering::SeqCst) {
            if queues.candidates.len() >= VoteGenerator::MAX_HASHES {
                queues = self.broadcast(queues);
            } else if let Some(request) = queues.requests.pop_front() {
                drop(queues);
                self.reply(request);
                queues = self.queues.lock().unwrap();
            } else {
                queues = self
                    .condition
                    .wait_timeout_while(queues, self.vote_generator_delay, |lk| {
                        lk.candidates.len() < VoteGenerator::MAX_HASHES
                    })
                    .unwrap()
                    .0;

                if queues.candidates.len() >= self.vote_generator_threshold
                    && queues.candidates.len() < VoteGenerator::MAX_HASHES
                {
                    queues = self
                        .condition
                        .wait_timeout_while(queues, self.vote_generator_delay, |lk| {
                            lk.candidates.len() < VoteGenerator::MAX_HASHES
                        })
                        .unwrap()
                        .0;
                }

                if !queues.candidates.is_empty() {
                    queues = self.broadcast(queues);
                }
            }
        }
    }

    fn broadcast<'a>(&'a self, mut queues: MutexGuard<'a, Queues>) -> MutexGuard<'a, Queues> {
        let mut hashes = Vec::with_capacity(VoteGenerator::MAX_HASHES);
        let mut roots = Vec::with_capacity(VoteGenerator::MAX_HASHES);
        {
            let spacing = self.spacing.lock().unwrap();
            while let Some((root, hash)) = queues.candidates.pop_front() {
                if !roots.contains(&root) {
                    if spacing.votable(&root, &hash) {
                        roots.push(root);
                        hashes.push(hash);
                    } else {
                        self.stats
                            .inc(StatType::VoteGenerator, DetailType::GeneratorSpacing);
                    }
                }
                if hashes.len() == VoteGenerator::MAX_HASHES {
                    break;
                }
            }
        }

        if !hashes.is_empty() {
            drop(queues);
            self.vote(&hashes, &roots, |vote| {
                self.vote_broadcaster.broadcast(vote);
                self.stats
                    .inc(StatType::VoteGenerator, DetailType::GeneratorBroadcasts);
            });
            queues = self.queues.lock().unwrap();
        }

        queues
    }

    fn vote<F>(&self, hashes: &Vec<BlockHash>, roots: &Vec<Root>, action: F)
    where
        F: Fn(Arc<Vote>),
    {
        debug_assert_eq!(hashes.len(), roots.len());
        let mut votes = Vec::new();
        self.wallets.foreach_representative(|keys| {
            let timestamp = if self.is_final {
                Vote::TIMESTAMP_MAX
            } else {
                milliseconds_since_epoch()
            };
            let duration = if self.is_final {
                Vote::DURATION_MAX
            } else {
                0x9 /*8192ms*/
            };
            votes.push(Arc::new(Vote::new(
                keys,
                timestamp,
                duration,
                hashes.clone(),
            )));
        });

        for vote in votes {
            {
                let mut spacing = self.spacing.lock().unwrap();
                for i in 0..hashes.len() {
                    self.history.add(&roots[i], &hashes[i], &vote);
                    spacing.flag(&roots[i], &hashes[i]);
                }
            }
            action(vote);
        }
    }

    fn reply(&self, request: (Vec<(Root, BlockHash)>, Arc<Channel>)) {
        let mut i = request.0.iter().peekable();
        while i.peek().is_some() && !self.stopped.load(Ordering::SeqCst) {
            let mut hashes = Vec::with_capacity(VoteGenerator::MAX_HASHES);
            let mut roots = Vec::with_capacity(VoteGenerator::MAX_HASHES);
            {
                let spacing = self.spacing.lock().unwrap();
                while hashes.len() < VoteGenerator::MAX_HASHES {
                    let Some((root, hash)) = i.next() else {
                        break;
                    };
                    if !roots.contains(root) {
                        if spacing.votable(root, hash) {
                            roots.push(*root);
                            hashes.push(*hash);
                        } else {
                            self.stats
                                .inc(StatType::VoteGenerator, DetailType::GeneratorSpacing);
                        }
                    }
                }
            }
            if !hashes.is_empty() {
                self.stats.add_dir(
                    StatType::Requests,
                    DetailType::RequestsGeneratedHashes,
                    Direction::In,
                    hashes.len() as u64,
                );
                self.vote(&hashes, &roots, |vote| {
                    let channel = &request.1;
                    let confirm =
                        Message::ConfirmAck(ConfirmAck::new_with_own_vote((*vote).clone()));
                    channel.try_send(&confirm, BufferDropPolicy::Limiter, TrafficType::Generic);
                    self.stats.inc_dir(
                        StatType::Requests,
                        DetailType::RequestsGeneratedVotes,
                        Direction::In,
                    );
                });
            }
        }
        self.stats
            .inc(StatType::VoteGenerator, DetailType::GeneratorReplies);
    }

    fn process_batch(&self, batch: VecDeque<(Root, BlockHash)>) {
        let mut verified = VecDeque::new();

        if self.is_final {
            let mut write_guard = self.ledger.write_queue.wait(Writer::VotingFinal);
            let mut tx = self.ledger.rw_txn();
            for (root, hash) in &batch {
                (write_guard, tx) = self.ledger.refresh_if_needed(write_guard, tx);
                if self.should_vote_final(&mut tx, root, hash) {
                    verified.push_back((*root, *hash));
                }
            }
        } else {
            let mut tx = self.ledger.read_txn();
            for (root, hash) in &batch {
                tx.refresh_if_needed();
                if self.should_vote_non_final(&tx, root, hash) {
                    verified.push_back((*root, *hash));
                }
            }
        };

        // Submit verified candidates to the main processing thread
        if !verified.is_empty() {
            let should_notify = {
                let mut queues = self.queues.lock().unwrap();
                queues.candidates.extend(verified);
                queues.candidates.len() >= VoteGenerator::MAX_HASHES
            };

            if should_notify {
                self.condition.notify_all();
            }
        }
    }

    fn should_vote_non_final(
        &self,
        txn: &LmdbReadTransaction,
        root: &Root,
        hash: &BlockHash,
    ) -> bool {
        let Some(block) = self.ledger.any().get_block(txn, hash) else {
            return false;
        };
        debug_assert!(block.root() == *root);
        self.ledger.dependents_confirmed(txn, &block)
    }

    fn should_vote_final(
        &self,
        txn: &mut LmdbWriteTransaction,
        root: &Root,
        hash: &BlockHash,
    ) -> bool {
        let Some(block) = self.ledger.any().get_block(txn, hash) else {
            return false;
        };
        debug_assert!(block.root() == *root);
        self.ledger.dependents_confirmed(txn, &block)
            && self
                .ledger
                .store
                .final_vote
                .put(txn, &block.qualified_root(), hash)
    }
}

#[derive(Default)]
struct Queues {
    candidates: VecDeque<(Root, BlockHash)>,
    requests: VecDeque<(Vec<(Root, BlockHash)>, Arc<Channel>)>,
}
