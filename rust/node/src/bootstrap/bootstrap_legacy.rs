use super::{
    BootstrapAttempt, BootstrapAttemptTrait, BootstrapConnections, BootstrapInitiator,
    BootstrapMode, BulkPushClient, BulkPushClientExt, FrontierReqClient, FrontierReqClientExt,
    PullInfo,
};
use crate::{
    block_processing::{BlockProcessor, BlockSource},
    bootstrap::BootstrapConnectionsExt,
    stats::{DetailType, Direction, StatType, Stats},
    utils::ThreadPool,
    websocket::WebsocketListener,
};
use rand::{thread_rng, Rng};
use rsnano_core::{utils::PropertyTree, Account, BlockEnum, BlockHash};
use rsnano_ledger::Ledger;
use std::{
    collections::VecDeque,
    net::{Ipv6Addr, SocketAddrV6},
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, Mutex, MutexGuard, Weak,
    },
    time::{Duration, Instant},
};
use tracing::debug;

pub struct LegacyBootstrapConfig {
    pub frontier_request_count: u32,
    pub frontier_retry_limit: u32,
    pub disable_bulk_push_client: bool,
}

/// Legacy bootstrap session. This is made up of 3 phases: frontier requests, bootstrap pulls, bootstrap pushes.
pub struct BootstrapAttemptLegacy {
    attempt: BootstrapAttempt,
    connections: Arc<BootstrapConnections>,
    mutex: Mutex<LegacyData>,
    config: LegacyBootstrapConfig,
    ledger: Arc<Ledger>,
    stats: Arc<Stats>,
    account_count: AtomicU32,
    block_processor: Weak<BlockProcessor>,
    workers: Arc<dyn ThreadPool>,
    tokio: tokio::runtime::Handle,
}

impl BootstrapAttemptLegacy {
    pub fn new(
        websocket_server: Option<Arc<WebsocketListener>>,
        block_processor: Weak<BlockProcessor>,
        bootstrap_initiator: Weak<BootstrapInitiator>,
        ledger: Arc<Ledger>,
        workers: Arc<dyn ThreadPool>,
        id: String,
        incremental_id: u64,
        connections: Arc<BootstrapConnections>,
        config: LegacyBootstrapConfig,
        stats: Arc<Stats>,
        tokio: tokio::runtime::Handle,
        frontiers_age: u32,
        start_account: Account,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            attempt: BootstrapAttempt::new(
                websocket_server,
                Weak::clone(&block_processor),
                bootstrap_initiator,
                Arc::clone(&ledger),
                id,
                BootstrapMode::Legacy,
                incremental_id,
            )?,
            connections,
            mutex: Mutex::new(LegacyData {
                endpoint_frontier_request: SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 0, 0, 0),
                frontiers_age,
                start_account,
                frontier_pulls: VecDeque::new(),
                push: None,
                frontiers: None,
                bulk_push_targets: Vec::new(),
            }),
            config,
            ledger,
            stats,
            tokio,
            account_count: AtomicU32::new(0),
            block_processor,
            workers,
        })
    }

    pub fn request_bulk_push_target(&self) -> Option<(BlockHash, BlockHash)> {
        let mut guard = self.mutex.lock().unwrap();
        guard.bulk_push_targets.pop()
    }

    pub fn add_frontier(&self, pull_info: PullInfo) {
        // Prevent incorrect or malicious pulls with frontier 0 insertion
        if !pull_info.head.is_zero() {
            let mut guard = self.mutex.lock().unwrap();
            guard.frontier_pulls.push_back(pull_info);
        }
    }

    pub fn set_start_account(&self, account: Account) {
        // Add last account fron frontier request
        let mut guard = self.mutex.lock().unwrap();
        guard.start_account = account;
    }

    pub fn add_bulk_push_target(&self, head: BlockHash, end: BlockHash) {
        let mut guard = self.mutex.lock().unwrap();
        guard.bulk_push_targets.push((head, end));
    }

    fn wait_until_block_processor_empty<'a>(
        &'a self,
        mut guard: MutexGuard<'a, LegacyData>,
        source: BlockSource,
    ) -> MutexGuard<'a, LegacyData> {
        let Some(processor) = self.block_processor.upgrade() else {
            return guard;
        };
        let wait_start = Instant::now();
        while !self.attempt.stopped()
            && processor.queue_len(source) > 0
            && wait_start.elapsed() < Duration::from_secs(10)
        {
            guard = self
                .attempt
                .condition
                .wait_timeout_while(guard, Duration::from_millis(100), |_| {
                    self.attempt.stopped() || processor.queue_len(source) == 0
                })
                .unwrap()
                .0
        }
        guard
    }
}

pub trait BootstrapAttemptLegacyExt {
    fn run_start<'a>(&'a self, guard: MutexGuard<'a, LegacyData>) -> MutexGuard<'a, LegacyData>;

    fn request_push<'a>(&'a self, guard: MutexGuard<'a, LegacyData>) -> MutexGuard<'a, LegacyData>;

    fn request_frontier<'a>(
        &'a self,
        lock_a: MutexGuard<'a, LegacyData>,
        first_attempt: bool,
    ) -> (MutexGuard<'a, LegacyData>, bool);
}

impl BootstrapAttemptLegacyExt for Arc<BootstrapAttemptLegacy> {
    fn run_start<'a>(
        &'a self,
        mut guard: MutexGuard<'a, LegacyData>,
    ) -> MutexGuard<'a, LegacyData> {
        self.attempt
            .frontiers_received
            .store(false, Ordering::SeqCst);
        let mut frontier_failure = true;
        let mut frontier_attempts = 0;
        while !self.attempt.stopped.load(Ordering::SeqCst) && frontier_failure {
            frontier_attempts += 1;
            (guard, frontier_failure) = self.request_frontier(guard, frontier_attempts == 1);
        }
        self.attempt
            .frontiers_received
            .store(true, Ordering::SeqCst);
        guard
    }

    fn request_push<'a>(
        &'a self,
        mut guard: MutexGuard<'a, LegacyData>,
    ) -> MutexGuard<'a, LegacyData> {
        let endpoint = guard.endpoint_frontier_request;
        drop(guard);
        let connection_l = self.connections.find_connection(endpoint);
        guard = self.mutex.lock().unwrap();
        if let Some(connection_l) = connection_l {
            let mut client = BulkPushClient::new(
                connection_l,
                self.ledger.clone(),
                self.tokio.clone(),
                self.workers.clone(),
            );
            client.set_attempt(self);
            let client = Arc::new(client);
            client.start();
            guard.push = Some(Arc::downgrade(&client));

            drop(guard);
            let _ = client.get_result();
            guard = self.mutex.lock().unwrap();
        }
        guard
    }

    fn request_frontier<'a>(
        &'a self,
        mut lock_a: MutexGuard<'a, LegacyData>,
        first_attempt: bool,
    ) -> (MutexGuard<'a, LegacyData>, bool) {
        let mut failure = true;
        drop(lock_a);
        let (connection_l, should_stop) = self.connections.connection(first_attempt);
        if should_stop {
            debug!("Bootstrap attempt stopped because there are no peers");
            self.attempt.stop();
        }

        lock_a = self.mutex.lock().unwrap();
        if let Some(connection_l) = connection_l {
            if !self.attempt.stopped() {
                lock_a.endpoint_frontier_request = connection_l.remote_addr();
                {
                    let mut client = FrontierReqClient::new(
                        connection_l.clone(),
                        self.ledger.clone(),
                        self.config.frontier_retry_limit,
                        self.connections.clone(),
                        self.workers.clone(),
                        self.tokio.clone(),
                    );
                    client.set_attempt(Arc::clone(self));
                    let client = Arc::new(client);
                    client.run(
                        &lock_a.start_account,
                        lock_a.frontiers_age,
                        self.config.frontier_request_count,
                    );
                    lock_a.frontiers = Some(Arc::downgrade(&client));
                    drop(lock_a);
                    failure = client.get_result();
                }
                lock_a = self.mutex.lock().unwrap();
                if failure {
                    lock_a.frontier_pulls.clear();
                } else {
                    self.account_count
                        .store(lock_a.frontier_pulls.len() as u32, Ordering::SeqCst);
                    // Shuffle pulls
                    assert!(u32::MAX as usize > lock_a.frontier_pulls.len());
                    if !lock_a.frontier_pulls.is_empty() {
                        let mut rng = thread_rng();
                        for i in (1..lock_a.frontier_pulls.len() - 1).rev() {
                            let k = rng.gen_range(0..=i);
                            lock_a.frontier_pulls.swap(i, k);
                        }
                    }
                    // Add to regular pulls
                    while !lock_a.frontier_pulls.is_empty() {
                        let pull = lock_a.frontier_pulls.front().unwrap().clone();
                        drop(lock_a);
                        self.connections.add_pull(pull);
                        lock_a = self.mutex.lock().unwrap();
                        self.attempt.pulling.fetch_add(1, Ordering::SeqCst);
                        lock_a.frontier_pulls.pop_front();
                    }
                }
                if !failure {
                    debug!(
                        "Completed frontier request, {} out of sync accounts according to {}",
                        self.account_count.load(Ordering::SeqCst),
                        connection_l.channel_string()
                    );
                } else {
                    self.stats
                        .inc_dir(StatType::Error, DetailType::FrontierReq, Direction::Out);
                }
            }
        }
        (lock_a, failure)
    }
}

pub struct LegacyData {
    endpoint_frontier_request: SocketAddrV6,
    start_account: Account,
    frontiers_age: u32,
    frontier_pulls: VecDeque<PullInfo>,
    push: Option<Weak<BulkPushClient>>,
    frontiers: Option<Weak<FrontierReqClient>>,
    bulk_push_targets: Vec<(BlockHash, BlockHash)>,
}
impl BootstrapAttemptTrait for Arc<BootstrapAttemptLegacy> {
    fn incremental_id(&self) -> u64 {
        self.attempt.incremental_id
    }

    fn id(&self) -> &str {
        &self.attempt.id
    }

    fn started(&self) -> bool {
        self.attempt.started.load(Ordering::SeqCst)
    }

    fn stopped(&self) -> bool {
        self.attempt.stopped()
    }

    fn stop(&self) {
        let guard = self.mutex.lock().unwrap();
        self.attempt.set_stopped();
        drop(guard);
        self.attempt.condition.notify_all();
        let guard = self.mutex.lock().unwrap();
        if let Some(frontiers) = &guard.frontiers {
            if let Some(frontiers) = frontiers.upgrade() {
                frontiers.set_result(true);
            }
        }

        if let Some(push) = &guard.push {
            if let Some(push) = push.upgrade() {
                push.set_result(true);
            }
        }
        drop(guard);
        if let Some(init) = self.attempt.bootstrap_initiator.upgrade() {
            init.clear_pulls(self.attempt.incremental_id);
        }
    }

    fn pull_finished(&self) {
        self.attempt.pull_finished();
    }

    fn pulling(&self) -> u32 {
        self.attempt.pulling.load(Ordering::SeqCst)
    }

    fn total_blocks(&self) -> u64 {
        self.attempt.total_blocks.load(Ordering::SeqCst)
    }

    fn inc_total_blocks(&self) {
        self.attempt.total_blocks.fetch_add(1, Ordering::SeqCst);
    }

    fn requeued_pulls(&self) -> u32 {
        self.attempt.requeued_pulls.load(Ordering::SeqCst)
    }

    fn inc_requeued_pulls(&self) {
        self.attempt.requeued_pulls.fetch_add(1, Ordering::SeqCst);
    }

    fn pull_started(&self) {
        self.attempt.pull_started();
    }

    fn duration(&self) -> Duration {
        self.attempt.duration()
    }

    fn set_started(&self) -> bool {
        !self.attempt.started.swap(true, Ordering::SeqCst)
    }

    fn should_log(&self) -> bool {
        self.attempt.should_log()
    }

    fn notify(&self) {
        self.attempt.condition.notify_all();
    }

    fn get_information(&self, tree: &mut dyn PropertyTree) -> anyhow::Result<()> {
        let guard = self.mutex.lock().unwrap();
        tree.put_string("frontier_pulls", &guard.frontier_pulls.len().to_string())?;
        tree.put_string(
            "frontiers_received",
            if self.attempt.frontiers_received.load(Ordering::SeqCst) {
                "true"
            } else {
                "false"
            },
        )?;
        tree.put_string("frontiers_age", &guard.frontiers_age.to_string())?;
        tree.put_string("last_account", &guard.start_account.encode_account())
    }

    fn run(&self) {
        debug_assert!(self.started());
        self.connections.populate_connections(false);
        let mut guard = self.mutex.lock().unwrap();
        guard = self.run_start(guard);
        while self.attempt.still_pulling() {
            while self.attempt.still_pulling() {
                while !(self.attempt.stopped() || self.pulling() == 0) {
                    guard = self.attempt.condition.wait(guard).unwrap();
                }
            }

            // TODO: This check / wait is a heuristic and should be improved.
            guard = self.wait_until_block_processor_empty(guard, BlockSource::BootstrapLegacy);

            if guard.start_account != Account::MAX {
                debug!(
                    "Requesting new frontiers after: {}",
                    guard.start_account.encode_account()
                );
                //
                // Requesting new frontiers
                guard = self.run_start(guard);
            }
        }
        if !self.attempt.stopped() {
            debug!("Completed legacy pulls");

            if !self.config.disable_bulk_push_client {
                guard = self.request_push(guard);
            }
        }
        drop(guard);
        self.attempt.stop();
        self.attempt.condition.notify_all();
    }

    fn process_block(
        &self,
        block: Arc<BlockEnum>,
        _known_account: &Account,
        pull_blocks_processed: u64,
        _max_blocks: u32,
        _block_expected: bool,
        _retry_limit: u32,
    ) -> bool {
        self.attempt.process_block(block, pull_blocks_processed)
    }
}
