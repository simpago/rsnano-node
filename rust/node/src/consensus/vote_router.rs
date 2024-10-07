use super::{Election, RecentlyConfirmedCache, VoteApplier};
use crate::consensus::VoteApplierExt;
use rsnano_core::{
    utils::{ContainerInfo, ContainerInfoComponent},
    BlockHash, Vote, VoteCode, VoteSource,
};
use std::{
    collections::HashMap,
    mem::size_of,
    sync::{Arc, Condvar, Mutex, Weak},
    thread::JoinHandle,
    time::Duration,
};

pub struct VoteRouter {
    thread: Mutex<Option<JoinHandle<()>>>,
    shared: Arc<(Condvar, Mutex<State>)>,
    vote_processed_observers: Mutex<Vec<VoteProcessedCallback>>,
    recently_confirmed: Arc<RecentlyConfirmedCache>,
    vote_applier: Arc<VoteApplier>,
}

impl VoteRouter {
    pub fn new(
        recently_confirmed: Arc<RecentlyConfirmedCache>,
        vote_applier: Arc<VoteApplier>,
    ) -> Self {
        Self {
            thread: Mutex::new(None),
            shared: Arc::new((
                Condvar::new(),
                Mutex::new(State {
                    stopped: false,
                    elections: HashMap::new(),
                }),
            )),
            vote_processed_observers: Mutex::new(Vec::new()),
            recently_confirmed,
            vote_applier,
        }
    }

    pub fn start(&self) {
        let shared = self.shared.clone();
        *self.thread.lock().unwrap() = Some(
            std::thread::Builder::new()
                .name("Voute router".to_owned())
                .spawn(move || {
                    let (condition, state) = &*shared;
                    let mut guard = state.lock().unwrap();
                    while !guard.stopped {
                        guard.clean_up();
                        guard = condition
                            .wait_timeout_while(guard, Duration::from_secs(15), |g| !g.stopped)
                            .unwrap()
                            .0;
                    }
                })
                .unwrap(),
        )
    }

    pub fn stop(&self) {
        self.shared.1.lock().unwrap().stopped = true;
        self.shared.0.notify_all();
        let thread = self.thread.lock().unwrap().take();
        if let Some(thread) = thread {
            thread.join().unwrap();
        }
    }

    pub fn add_vote_processed_observer(&self, observer: VoteProcessedCallback) {
        self.vote_processed_observers.lock().unwrap().push(observer);
    }

    pub fn connect(&self, hash: BlockHash, election: Weak<Election>) {
        self.shared
            .1
            .lock()
            .unwrap()
            .elections
            .insert(hash, election);
    }

    pub fn disconnect_election(&self, election: &Election) {
        let mut state = self.shared.1.lock().unwrap();
        let election_guard = election.mutex.lock().unwrap();
        for hash in election_guard.last_blocks.keys() {
            state.elections.remove(hash);
        }
    }

    pub fn disconnect(&self, hash: &BlockHash) {
        let mut state = self.shared.1.lock().unwrap();
        state.elections.remove(hash);
    }

    pub fn election(&self, hash: &BlockHash) -> Option<Arc<Election>> {
        let state = self.shared.1.lock().unwrap();
        state.elections.get(hash)?.upgrade()
    }

    /// Route vote to associated elections
    /// Distinguishes replay votes, cannot be determined if the block is not in any election
    /// If 'filter' parameter is non-zero, only elections for the specified hash are notified.
    /// This eliminates duplicate processing when triggering votes from the vote_cache as the result of a specific election being created.
    pub fn vote_filter(
        &self,
        vote: &Arc<Vote>,
        source: VoteSource,
        filter: &BlockHash,
    ) -> HashMap<BlockHash, VoteCode> {
        debug_assert!(vote.validate().is_ok());
        // If present, filter should be set to one of the hashes in the vote
        debug_assert!(filter.is_zero() || vote.hashes.iter().any(|h| h == filter));

        let mut results = HashMap::new();
        let mut process = HashMap::new();
        {
            let guard = self.shared.1.lock().unwrap();
            for hash in &vote.hashes {
                // Ignore votes for other hashes if a filter is set
                if !filter.is_zero() && hash != filter {
                    continue;
                }

                // Ignore duplicate hashes (should not happen with a well-behaved voting node)
                if results.contains_key(hash) {
                    continue;
                }

                let election = guard.elections.get(hash).and_then(|e| e.upgrade());
                if let Some(election) = election {
                    process.insert(*hash, election.clone());
                } else {
                    if !self.recently_confirmed.hash_exists(hash) {
                        results.insert(*hash, VoteCode::Indeterminate);
                    } else {
                        results.insert(*hash, VoteCode::Replay);
                    }
                }
            }
        }

        for (block_hash, election) in process {
            let vote_result = self.vote_applier.vote(
                &election,
                &vote.voting_account,
                vote.timestamp(),
                &block_hash,
                source,
            );
            results.insert(block_hash, vote_result);
        }

        self.on_vote_processed(vote, source, &results);

        results
    }

    /// Route vote to associated elections
    /// Distinguishes replay votes, cannot be determined if the block is not in any election
    pub fn vote(&self, vote: &Arc<Vote>, source: VoteSource) -> HashMap<BlockHash, VoteCode> {
        self.vote_filter(vote, source, &BlockHash::zero())
    }

    pub fn active(&self, hash: &BlockHash) -> bool {
        let state = self.shared.1.lock().unwrap();
        if let Some(existing) = state.elections.get(hash) {
            existing.strong_count() > 0
        } else {
            false
        }
    }

    fn on_vote_processed(
        &self,
        vote: &Arc<Vote>,
        source: VoteSource,
        results: &HashMap<BlockHash, VoteCode>,
    ) {
        let observers = self.vote_processed_observers.lock().unwrap();
        for o in observers.iter() {
            o(vote, source, results);
        }
    }

    pub fn collect_container_info(&self, name: impl Into<String>) -> ContainerInfoComponent {
        let guard = self.shared.1.lock().unwrap();
        ContainerInfoComponent::Composite(
            name.into(),
            vec![ContainerInfoComponent::Leaf(ContainerInfo {
                name: "elections".to_owned(),
                count: guard.elections.len(),
                sizeof_element: size_of::<BlockHash>() + size_of::<Weak<Election>>(),
            })],
        )
    }
}

impl Drop for VoteRouter {
    fn drop(&mut self) {
        // Thread must be stopped before destruction
        debug_assert!(self.thread.lock().unwrap().is_none())
    }
}

struct State {
    stopped: bool,
    // Mapping of block hashes to elections.
    // Election already contains the associated block
    elections: HashMap<BlockHash, Weak<Election>>,
}

impl State {
    fn clean_up(&mut self) {
        self.elections
            .retain(|_, election| election.strong_count() > 0)
    }
}

pub type VoteProcessedCallback =
    Box<dyn Fn(&Arc<Vote>, VoteSource, &HashMap<BlockHash, VoteCode>) + Send + Sync>;
