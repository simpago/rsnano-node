use super::{VoteCache, VoteProcessorConfig, VoteRouter};
use crate::stats::{DetailType, StatType, Stats};
use rsnano_core::{utils::ContainerInfo, BlockHash, VoteSource};
use std::{
    collections::{HashSet, VecDeque},
    sync::{Arc, Condvar, Mutex, MutexGuard},
    thread::JoinHandle,
};

pub(crate) struct VoteCacheProcessor {
    state: Arc<Mutex<State>>,
    condition: Arc<Condvar>,
    stats: Arc<Stats>,
    vote_cache: Arc<Mutex<VoteCache>>,
    vote_router: Arc<VoteRouter>,
    config: VoteProcessorConfig,
}

impl VoteCacheProcessor {
    pub(crate) fn new(
        stats: Arc<Stats>,
        vote_cache: Arc<Mutex<VoteCache>>,
        vote_router: Arc<VoteRouter>,
        config: VoteProcessorConfig,
    ) -> Self {
        Self {
            state: Arc::new(Mutex::new(State {
                thread: None,
                stopped: false,
                triggered: VecDeque::new(),
            })),
            condition: Arc::new(Condvar::new()),
            stats,
            vote_router,
            vote_cache,
            config,
        }
    }
}

impl VoteCacheProcessor {
    pub fn start(&self) {
        debug_assert!(self.state.lock().unwrap().thread.is_none());
        let cache_loop = VoteCacheLoop {
            state: self.state.clone(),
            condition: self.condition.clone(),
            stats: self.stats.clone(),
            vote_cache: self.vote_cache.clone(),
            vote_router: self.vote_router.clone(),
        };

        self.state.lock().unwrap().thread = Some(
            std::thread::Builder::new()
                .name("Vote cache proc".to_owned())
                .spawn(move || cache_loop.run())
                .unwrap(),
        );
    }

    pub fn stop(&self) {
        let thread = {
            let mut state = self.state.lock().unwrap();
            state.stopped = true;
            state.thread.take()
        };

        self.condition.notify_all();

        if let Some(handle) = thread {
            handle.join().unwrap();
        }
    }

    pub fn trigger(&self, hash: BlockHash) {
        {
            let mut state = self.state.lock().unwrap();
            if state.triggered.len() > self.config.max_triggered {
                state.triggered.pop_front();
                self.stats
                    .inc(StatType::VoteCacheProcessor, DetailType::Overfill);
            }
            state.triggered.push_back(hash);
        }
        self.condition.notify_all();
        self.stats
            .inc(StatType::VoteCacheProcessor, DetailType::Triggered);
    }

    pub fn len(&self) -> usize {
        self.state.lock().unwrap().triggered.len()
    }

    pub fn container_info(&self) -> ContainerInfo {
        [("triggered", self.len(), std::mem::size_of::<BlockHash>())].into()
    }
}

impl Drop for VoteCacheProcessor {
    fn drop(&mut self) {
        debug_assert!(self.state.lock().unwrap().thread.is_none())
    }
}

struct State {
    thread: Option<JoinHandle<()>>,
    stopped: bool,
    triggered: VecDeque<BlockHash>,
}

struct VoteCacheLoop {
    state: Arc<Mutex<State>>,
    condition: Arc<Condvar>,
    stats: Arc<Stats>,
    vote_cache: Arc<Mutex<VoteCache>>,
    vote_router: Arc<VoteRouter>,
}

impl VoteCacheLoop {
    fn run(&self) {
        let mut guard = self.state.lock().unwrap();
        while !guard.stopped {
            if !guard.triggered.is_empty() {
                self.run_batch(guard);
                guard = self.state.lock().unwrap();
            } else {
                guard = self
                    .condition
                    .wait_while(guard, |i| !i.stopped && i.triggered.is_empty())
                    .unwrap();
            }
        }
    }

    fn run_batch(&self, mut state: MutexGuard<'_, State>) {
        let mut triggered = VecDeque::new();
        std::mem::swap(&mut triggered, &mut state.triggered);
        drop(state);

        //deduplicate
        let hashes: HashSet<BlockHash> = triggered.drain(..).collect();

        self.stats.add(
            StatType::VoteCacheProcessor,
            DetailType::Processed,
            hashes.len() as u64,
        );

        for hash in hashes {
            let cached = self.vote_cache.lock().unwrap().find(&hash);
            for cached_vote in cached {
                self.vote_router
                    .vote_filter(&cached_vote, VoteSource::Cache, &hash);
            }
        }
    }
}
