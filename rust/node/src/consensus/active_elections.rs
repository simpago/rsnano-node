use super::{
    confirmation_solicitor::ConfirmationSolicitor, Election, ElectionBehavior, ElectionData,
    ElectionState, ElectionStatus, ElectionStatusType, RecentlyConfirmedCache, VoteApplier,
    VoteCache, VoteCacheProcessor, VoteGenerators, VoteRouter, NEXT_ELECTION_ID,
};
use crate::{
    block_processing::BlockProcessor,
    cementation::ConfirmingSet,
    config::{NodeConfig, NodeFlags},
    consensus::VoteApplierExt,
    representatives::OnlineReps,
    stats::{DetailType, Direction, Sample, StatType, Stats},
    transport::{BufferDropPolicy, Network},
    utils::HardenedConstants,
    wallets::Wallets,
    NetworkParams,
};
use bounded_vec_deque::BoundedVecDeque;
use rsnano_core::{
    utils::{ContainerInfo, ContainerInfoComponent, MemoryStream, TomlWriter},
    Account, Amount, BlockEnum, BlockHash, BlockType, QualifiedRoot, Vote, VoteWithWeightInfo,
};
use rsnano_ledger::{BlockStatus, Ledger};
use rsnano_messages::{Message, Publish};
use rsnano_store_lmdb::{LmdbReadTransaction, Transaction};
use serde::{Deserialize, Serialize};
use std::{
    cmp::max,
    collections::{BTreeMap, HashMap},
    mem::size_of,
    ops::Deref,
    sync::{atomic::Ordering, Arc, Condvar, Mutex, MutexGuard},
    thread::JoinHandle,
    time::{Duration, Instant},
};
use tracing::{debug, trace};

const ELECTION_MAX_BLOCKS: usize = 10;

pub type ElectionEndCallback = Box<
    dyn Fn(&ElectionStatus, &Vec<VoteWithWeightInfo>, Account, Amount, bool, bool) + Send + Sync,
>;

pub type AccountBalanceChangedCallback = Box<dyn Fn(&Account, bool) + Send + Sync>;

#[derive(Clone, Debug)]
pub struct ActiveElectionsConfig {
    // Maximum number of simultaneous active elections (AEC size)
    pub size: usize,
    // Limit of hinted elections as percentage of `active_elections_size`
    pub hinted_limit_percentage: usize,
    // Limit of optimistic elections as percentage of `active_elections_size`
    pub optimistic_limit_percentage: usize,
    // Maximum confirmation history size
    pub confirmation_history_size: usize,
    // Maximum cache size for recently_confirmed
    pub confirmation_cache: usize,
}

impl ActiveElectionsConfig {
    pub(crate) fn serialize_toml(&self, toml: &mut dyn TomlWriter) -> anyhow::Result<()> {
        toml.put_usize ("size", self.size, "Number of active elections. Elections beyond this limit have limited survival time.\nWarning: modifying this value may result in a lower confirmation rate. \ntype:uint64,[250..]")?;

        toml.put_usize(
            "hinted_limit_percentage",
            self.hinted_limit_percentage,
            "Limit of hinted elections as percentage of `active_elections_size` \ntype:uint64",
        )?;

        toml.put_usize(
            "optimistic_limit_percentage",
            self.optimistic_limit_percentage,
            "Limit of optimistic elections as percentage of `active_elections_size`. \ntype:uint64",
        )?;

        toml.put_usize ("confirmation_history_size", self.confirmation_history_size, "Maximum confirmation history size. If tracking the rate of block confirmations, the websocket feature is recommended instead. \ntype:uint64")?;

        toml.put_usize ("confirmation_cache", self.confirmation_cache, "Maximum number of confirmed elections kept in cache to prevent restarting an election. \ntype:uint64")
    }
}

impl Default for ActiveElectionsConfig {
    fn default() -> Self {
        Self {
            size: 5000,
            hinted_limit_percentage: 20,
            optimistic_limit_percentage: 10,
            confirmation_history_size: 2048,
            confirmation_cache: 65536,
        }
    }
}

pub struct ActiveElections {
    relative_time: Instant,
    pub mutex: Mutex<ActiveElectionsState>,
    pub condition: Condvar,
    network_params: NetworkParams,
    wallets: Arc<Wallets>,
    node_config: NodeConfig,
    config: ActiveElectionsConfig,
    ledger: Arc<Ledger>,
    confirming_set: Arc<ConfirmingSet>,
    pub recently_confirmed: Arc<RecentlyConfirmedCache>,
    /// Helper container for storing recently cemented elections (a block from election might be confirmed but not yet cemented by confirmation height processor)
    pub recently_cemented: Arc<Mutex<BoundedVecDeque<ElectionStatus>>>,
    block_processor: Arc<BlockProcessor>,
    vote_generators: Arc<VoteGenerators>,
    network: Arc<Network>,
    pub vacancy_update: Mutex<Box<dyn Fn() + Send + Sync>>,
    vote_cache: Arc<Mutex<VoteCache>>,
    stats: Arc<Stats>,
    active_started_observer: Mutex<Vec<Box<dyn Fn(BlockHash) + Send + Sync>>>,
    active_stopped_observer: Mutex<Vec<Box<dyn Fn(BlockHash) + Send + Sync>>>,
    activate_successors: Mutex<Box<dyn Fn(&LmdbReadTransaction, &Arc<BlockEnum>) + Send + Sync>>,
    election_end: Mutex<Vec<ElectionEndCallback>>,
    account_balance_changed: AccountBalanceChangedCallback,
    online_reps: Arc<Mutex<OnlineReps>>,
    thread: Mutex<Option<JoinHandle<()>>>,
    flags: NodeFlags,
    pub vote_applier: Arc<VoteApplier>,
    pub vote_router: Arc<VoteRouter>,
    vote_cache_processor: Arc<VoteCacheProcessor>,
}

impl ActiveElections {
    pub(crate) fn new(
        network_params: NetworkParams,
        wallets: Arc<Wallets>,
        node_config: NodeConfig,
        ledger: Arc<Ledger>,
        confirming_set: Arc<ConfirmingSet>,
        block_processor: Arc<BlockProcessor>,
        vote_generators: Arc<VoteGenerators>,
        network: Arc<Network>,
        vote_cache: Arc<Mutex<VoteCache>>,
        stats: Arc<Stats>,
        election_end: ElectionEndCallback,
        account_balance_changed: AccountBalanceChangedCallback,
        online_reps: Arc<Mutex<OnlineReps>>,
        flags: NodeFlags,
        recently_confirmed: Arc<RecentlyConfirmedCache>,
        vote_applier: Arc<VoteApplier>,
        vote_router: Arc<VoteRouter>,
        vote_cache_processor: Arc<VoteCacheProcessor>,
        relative_time: Instant,
    ) -> Self {
        Self {
            mutex: Mutex::new(ActiveElectionsState {
                roots: OrderedRoots::default(),
                stopped: false,
                manual_count: 0,
                priority_count: 0,
                hinted_count: 0,
                optimistic_count: 0,
            }),
            condition: Condvar::new(),
            network_params,
            wallets,
            ledger,
            confirming_set,
            recently_confirmed,
            recently_cemented: Arc::new(Mutex::new(BoundedVecDeque::new(
                node_config.active_elections.confirmation_history_size,
            ))),
            config: node_config.active_elections.clone(),
            node_config,
            block_processor,
            vote_generators,
            network,
            vacancy_update: Mutex::new(Box::new(|| {})),
            vote_cache,
            stats,
            active_started_observer: Mutex::new(Vec::new()),
            active_stopped_observer: Mutex::new(Vec::new()),
            activate_successors: Mutex::new(Box::new(|_tx, _block| {})),
            election_end: Mutex::new(vec![election_end]),
            account_balance_changed,
            online_reps,
            thread: Mutex::new(None),
            flags,
            vote_applier,
            vote_router,
            vote_cache_processor,
            relative_time,
        }
    }

    pub fn len(&self) -> usize {
        self.mutex.lock().unwrap().roots.len()
    }

    pub(crate) fn info(&self) -> ActiveElectionsInfo {
        let guard = self.mutex.lock().unwrap();
        ActiveElectionsInfo {
            total: guard.roots.len(),
            priority: guard.priority_count,
            hinted: guard.hinted_count,
            optimistic: guard.optimistic_count,
        }
    }

    pub fn add_election_end_callback(&self, f: ElectionEndCallback) {
        self.election_end.lock().unwrap().push(f);
    }

    pub fn add_active_started_callback(&self, f: Box<dyn Fn(BlockHash) + Send + Sync>) {
        self.active_started_observer.lock().unwrap().push(f);
    }

    pub fn add_active_stopped_callback(&self, f: Box<dyn Fn(BlockHash) + Send + Sync>) {
        self.active_stopped_observer.lock().unwrap().push(f);
    }

    pub fn clear_recently_confirmed(&self) {
        self.recently_confirmed.clear();
    }

    pub fn recently_confirmed_count(&self) -> usize {
        self.recently_confirmed.len()
    }

    pub fn recently_cemented_count(&self) -> usize {
        self.recently_cemented.lock().unwrap().len()
    }

    pub fn was_recently_confirmed(&self, hash: &BlockHash) -> bool {
        self.recently_confirmed.hash_exists(hash)
    }

    pub fn latest_recently_confirmed(&self) -> Option<(QualifiedRoot, BlockHash)> {
        self.recently_confirmed.back()
    }

    pub fn insert_recently_confirmed(&self, block: &BlockEnum) {
        self.recently_confirmed
            .put(block.qualified_root(), block.hash());
    }

    pub fn insert_recently_cemented(&self, status: ElectionStatus) {
        self.recently_cemented
            .lock()
            .unwrap()
            .push_back(status.clone());

        // Trigger callback for confirmed block
        let block = status.winner.as_ref().unwrap();
        let account = block.account();
        let amount = self
            .ledger
            .any()
            .block_amount(&self.ledger.read_txn(), &block.hash());
        let mut is_state_send = false;
        let mut is_state_epoch = false;
        if amount.is_some() {
            if block.block_type() == BlockType::State {
                is_state_send = block.is_send();
                is_state_epoch = block.is_epoch();
            }
        }

        let callbacks = self.election_end.lock().unwrap();
        for callback in callbacks.iter() {
            (callback)(
                &status,
                &Vec::new(),
                account,
                amount.unwrap_or_default(),
                is_state_send,
                is_state_epoch,
            );
        }
    }

    pub fn recently_cemented_list(&self) -> BoundedVecDeque<ElectionStatus> {
        self.recently_cemented.lock().unwrap().clone()
    }

    /*
     * Callbacks
     */
    pub fn set_activate_successors_callback(
        &self,
        callback: Box<dyn Fn(&LmdbReadTransaction, &Arc<BlockEnum>) + Send + Sync>,
    ) {
        *self.activate_successors.lock().unwrap() = callback;
    }

    //--------------------------------------------------------------------------------

    pub fn notify_observers(
        &self,
        tx: &LmdbReadTransaction,
        status: &ElectionStatus,
        votes: &Vec<VoteWithWeightInfo>,
    ) {
        let block = status.winner.as_ref().unwrap();
        let account = block.account();

        match status.election_status_type {
            ElectionStatusType::ActiveConfirmedQuorum => self.stats.inc_dir(
                StatType::ConfirmationObserver,
                DetailType::ActiveQuorum,
                Direction::Out,
            ),
            ElectionStatusType::ActiveConfirmationHeight => self.stats.inc_dir(
                StatType::ConfirmationObserver,
                DetailType::ActiveConfHeight,
                Direction::Out,
            ),
            ElectionStatusType::InactiveConfirmationHeight => self.stats.inc_dir(
                StatType::ConfirmationObserver,
                DetailType::InactiveConfHeight,
                Direction::Out,
            ),
            _ => {}
        }

        if !self.election_end.lock().unwrap().is_empty() {
            let amount = self
                .ledger
                .any()
                .block_amount(tx, &block.hash())
                .unwrap_or_default();

            let is_state_send = block.block_type() == BlockType::State && block.is_send();
            let is_state_epoch = block.block_type() == BlockType::State && block.is_epoch();

            let ended_callbacks = self.election_end.lock().unwrap();
            for callback in ended_callbacks.iter() {
                (callback)(
                    status,
                    votes,
                    account,
                    amount,
                    is_state_send,
                    is_state_epoch,
                );
            }
        }

        (self.account_balance_changed)(&account, false);
        if block.is_send() {
            (self.account_balance_changed)(&block.destination().unwrap(), true);
        }
    }

    pub fn request_loop2<'a>(
        &self,
        stamp: Instant,
        guard: MutexGuard<'a, ActiveElectionsState>,
    ) -> MutexGuard<'a, ActiveElectionsState> {
        if !guard.stopped {
            let loop_interval = self.network_params.network.aec_loop_interval;
            let min_sleep = loop_interval / 2;

            let wait_duration = max(
                min_sleep,
                (stamp + loop_interval).saturating_duration_since(Instant::now()),
            );

            self.condition
                .wait_timeout_while(guard, wait_duration, |data| !data.stopped)
                .unwrap()
                .0
        } else {
            guard
        }
    }

    pub fn remove_block(&self, election_guard: &mut MutexGuard<ElectionData>, hash: &BlockHash) {
        if election_guard.status.winner.as_ref().unwrap().hash() != *hash {
            if let Some(existing) = election_guard.last_blocks.remove(hash) {
                election_guard.last_votes.retain(|_, v| v.hash != *hash);
                self.clear_publish_filter(&existing);
            }
        }
    }

    fn clear_publish_filter(&self, block: &BlockEnum) {
        let mut buf = MemoryStream::new();
        block.serialize_without_block_type(&mut buf);
        self.network.publish_filter.clear_bytes(buf.as_bytes());
    }

    /// Maximum number of elections that should be present in this container
    /// NOTE: This is only a soft limit, it is possible for this container to exceed this count
    pub fn limit(&self, behavior: ElectionBehavior) -> usize {
        match behavior {
            ElectionBehavior::Manual => usize::MAX,
            ElectionBehavior::Priority => self.config.size,
            ElectionBehavior::Hinted => {
                self.config.hinted_limit_percentage * self.config.size / 100
            }
            ElectionBehavior::Optimistic => {
                self.config.optimistic_limit_percentage * self.config.size / 100
            }
        }
    }

    /// How many election slots are available for specified election type
    pub fn vacancy(&self, behavior: ElectionBehavior) -> i64 {
        let guard = self.mutex.lock().unwrap();
        match behavior {
            ElectionBehavior::Manual => i64::MAX,
            ElectionBehavior::Priority => {
                self.limit(ElectionBehavior::Priority) as i64 - guard.roots.len() as i64
            }
            ElectionBehavior::Hinted | ElectionBehavior::Optimistic => {
                self.limit(behavior) as i64 - guard.count_by_behavior(behavior) as i64
            }
        }
    }

    pub fn clear(&self) {
        // TODO: Call erased_callback for each election
        {
            let mut guard = self.mutex.lock().unwrap();
            guard.roots.clear();
        }

        (self.vacancy_update.lock().unwrap())()
    }

    pub fn active_root(&self, root: &QualifiedRoot) -> bool {
        let guard = self.mutex.lock().unwrap();
        guard.roots.get(root).is_some()
    }

    pub fn active(&self, block: &BlockEnum) -> bool {
        let guard = self.mutex.lock().unwrap();
        guard.roots.get(&block.qualified_root()).is_some()
    }

    pub fn replace_by_weight<'a>(
        &self,
        election: &'a Election,
        mut election_guard: MutexGuard<'a, ElectionData>,
        hash: &BlockHash,
    ) -> (bool, MutexGuard<'a, ElectionData>) {
        let mut replaced_block = BlockHash::zero();
        let winner_hash = election_guard.status.winner.as_ref().unwrap().hash();
        // Sort existing blocks tally
        let mut sorted: Vec<_> = election_guard
            .last_tally
            .iter()
            .map(|(hash, amount)| (*hash, *amount))
            .collect();
        drop(election_guard);

        // Sort in ascending order
        sorted.sort_by(|left, right| right.cmp(left));

        let votes_tally = |votes: &[Arc<Vote>]| {
            let mut result = Amount::zero();
            for vote in votes {
                result += self.ledger.weight(&vote.voting_account);
            }
            result
        };

        // Replace if lowest tally is below inactive cache new block weight
        let inactive_existing = self.vote_cache.lock().unwrap().find(hash);
        let inactive_tally = votes_tally(&inactive_existing);
        if inactive_tally > Amount::zero() && sorted.len() < ELECTION_MAX_BLOCKS {
            // If count of tally items is less than 10, remove any block without tally
            election_guard = election.mutex.lock().unwrap();
            for (hash, _) in &election_guard.last_blocks {
                if sorted.iter().all(|(h, _)| h != hash) && *hash != winner_hash {
                    replaced_block = *hash;
                    break;
                }
            }
        } else if inactive_tally > Amount::zero() && inactive_tally > sorted.first().unwrap().1 {
            if sorted.first().unwrap().0 != winner_hash {
                replaced_block = sorted[0].0;
            } else if inactive_tally > sorted[1].1 {
                // Avoid removing winner
                replaced_block = sorted[1].0;
            }
        }

        let mut replaced = false;
        if !replaced_block.is_zero() {
            self.vote_router.disconnect(&replaced_block);
            election_guard = election.mutex.lock().unwrap();
            self.remove_block(&mut election_guard, &replaced_block);
            replaced = true;
        } else {
            election_guard = election.mutex.lock().unwrap();
        }
        (replaced, election_guard)
    }

    pub fn publish(&self, block: &Arc<BlockEnum>, election: &Election) -> bool {
        let mut election_guard = election.mutex.lock().unwrap();

        // Do not insert new blocks if already confirmed
        let mut result = election_guard.is_confirmed();
        if !result
            && election_guard.last_blocks.len() >= ELECTION_MAX_BLOCKS
            && !election_guard.last_blocks.contains_key(&block.hash())
        {
            let (replaced, guard) = self.replace_by_weight(election, election_guard, &block.hash());
            election_guard = guard;
            if !replaced {
                result = true;
                self.clear_publish_filter(block);
            }
        }
        if !result {
            if election_guard.last_blocks.get(&block.hash()).is_some() {
                result = true;
                election_guard
                    .last_blocks
                    .insert(block.hash(), Arc::clone(block));
                if election_guard.status.winner.as_ref().unwrap().hash() == block.hash() {
                    election_guard.status.winner = Some(Arc::clone(block));
                    let message = Message::Publish(Publish::new_forward(block.as_ref().clone()));
                    self.network
                        .flood_message2(&message, BufferDropPolicy::NoLimiterDrop, 1.0);
                }
            } else {
                election_guard
                    .last_blocks
                    .insert(block.hash(), Arc::clone(block));
            }
        }
        /*
        Result is true if:
        1) election is confirmed or expired
        2) given election contains 10 blocks & new block didn't receive enough votes to replace existing blocks
        3) given block in already in election & election contains less than 10 blocks (replacing block content with new)
        */
        result
    }

    /// Broadcasts vote for the current winner of this election
    /// Checks if sufficient amount of time (`vote_generation_interval`) passed since the last vote generation
    pub fn broadcast_vote(
        &self,
        election: &Election,
        election_guard: &mut MutexGuard<ElectionData>,
    ) {
        if election_guard.last_vote_elapsed() >= self.network_params.network.vote_broadcast_interval
        {
            self.broadcast_vote_locked(election_guard, election);
            election_guard.set_last_vote();
        }
    }

    pub fn broadcast_block(
        &self,
        solicitor: &mut ConfirmationSolicitor,
        election: &Election,
        election_guard: &mut MutexGuard<ElectionData>,
    ) {
        if self.broadcast_block_predicate(election, election_guard) {
            if solicitor.broadcast(election_guard).is_ok() {
                let last_block_hash = election_guard.last_block_hash;
                self.stats.inc(
                    StatType::Election,
                    if last_block_hash.is_zero() {
                        DetailType::BroadcastBlockInitial
                    } else {
                        DetailType::BroadcastBlockRepeat
                    },
                );
                election.set_last_block();
                election_guard.last_block_hash =
                    election_guard.status.winner.as_ref().unwrap().hash();
            }
        }
    }

    /// Broadcast vote for current election winner. Generates final vote if reached quorum or already confirmed
    /// Requires mutex lock
    pub fn broadcast_vote_locked(
        &self,
        election_guard: &mut MutexGuard<ElectionData>,
        election: &Election,
    ) {
        let last_vote_elapsed = election_guard.last_vote_elapsed();
        if last_vote_elapsed < self.network_params.network.vote_broadcast_interval {
            return;
        }
        election_guard.set_last_vote();
        if self.node_config.enable_voting && self.wallets.voting_reps_count() > 0 {
            self.stats
                .inc(StatType::Election, DetailType::BroadcastVote);

            if election_guard.is_confirmed()
                || self
                    .vote_applier
                    .have_quorum(&self.vote_applier.tally_impl(election_guard))
            {
                self.stats
                    .inc(StatType::Election, DetailType::GenerateVoteFinal);
                let winner = election_guard.status.winner.as_ref().unwrap().hash();
                trace!(qualified_root = ?election.qualified_root, %winner, "type" = "final", "broadcast vote");
                self.vote_generators
                    .generate_final_vote(&election.root, &winner); // Broadcasts vote to the network
            } else {
                self.stats
                    .inc(StatType::Election, DetailType::GenerateVoteNormal);
                let winner = election_guard.status.winner.as_ref().unwrap().hash();
                trace!(qualified_root = ?election.qualified_root, %winner, "type" = "normal", "broadcast vote");
                self.vote_generators
                    .generate_non_final_vote(&election.root, &winner); // Broadcasts vote to the network
            }
        }
    }

    /// Erase all blocks from active and, if not confirmed, clear digests from network filters
    pub fn cleanup_election<'a>(
        &self,
        mut guard: MutexGuard<'a, ActiveElectionsState>,
        election: &'a Arc<Election>,
    ) {
        // Keep track of election count by election type
        debug_assert!(guard.count_by_behavior(election.behavior) > 0);
        *guard.count_by_behavior_mut(election.behavior) -= 1;

        let election_winner: BlockHash;
        let election_state;
        let blocks;
        {
            let election_guard = election.mutex.lock().unwrap();
            blocks = election_guard.last_blocks.clone();
            election_winner = election_guard.status.winner.as_ref().unwrap().hash();
            election_state = election_guard.state;
        }

        self.vote_router.disconnect_election(election);

        // Erase root info
        let entry = guard
            .roots
            .erase(&election.qualified_root)
            .expect("election not found");

        let state = election.state();
        self.stats
            .inc(StatType::ActiveElections, DetailType::Stopped);
        self.stats.inc(
            StatType::ActiveElections,
            if state.is_confirmed() {
                DetailType::Confirmed
            } else {
                DetailType::Unconfirmed
            },
        );
        self.stats
            .inc(StatType::ActiveElectionsStopped, state.into());
        self.stats.inc(state.into(), election.behavior.into());

        trace!(election = ?election, "active stopped");

        debug!(
            "Erased election for blocks: {} (behavior: {:?}, state: {:?})",
            blocks
                .keys()
                .map(|k| k.to_string())
                .collect::<Vec<_>>()
                .join(", "),
            election.behavior,
            election_state
        );

        drop(guard);

        // Track election duration
        self.stats.sample(
            Sample::ActiveElectionDuration,
            election.duration().as_millis() as i64,
            (0, 1000 * 60 * 10),
        ); // 0-10 minutes range

        // Notify observers without holding the lock
        if let Some(callback) = entry.erased_callback {
            callback(election)
        }

        (self.vacancy_update.lock().unwrap())();

        for (hash, block) in blocks {
            // Notify observers about dropped elections & blocks lost confirmed elections
            if !self.confirmed(election) || hash != election_winner {
                let callbacks = self.active_stopped_observer.lock().unwrap();
                for callback in callbacks.iter() {
                    (callback)(hash);
                }
            }

            if !self.confirmed(election) {
                // Clear from publish filter
                self.clear_publish_filter(&block);
            }
        }
    }

    pub fn confirmed(&self, election: &Election) -> bool {
        election.mutex.lock().unwrap().is_confirmed()
    }

    /// Minimum time between broadcasts of the current winner of an election, as a backup to requesting confirmations
    fn base_latency(&self) -> Duration {
        if self.network_params.network.is_dev_network() {
            Duration::from_millis(25)
        } else {
            Duration::from_millis(1000)
        }
    }

    /// Calculates time delay between broadcasting confirmation requests
    pub fn confirm_req_time(&self, election: &Election) -> Duration {
        match election.behavior {
            ElectionBehavior::Priority | ElectionBehavior::Manual | ElectionBehavior::Hinted => {
                self.base_latency() * 5
            }
            ElectionBehavior::Optimistic => self.base_latency() * 2,
        }
    }

    pub fn broadcast_block_predicate(
        &self,
        election: &Election,
        election_guard: &MutexGuard<ElectionData>,
    ) -> bool {
        // Broadcast the block if enough time has passed since the last broadcast (or it's the first broadcast)
        if election.last_block_elapsed() < self.network_params.network.block_broadcast_interval {
            true
        }
        // Or the current election winner has changed
        else if election_guard.status.winner.as_ref().unwrap().hash()
            != election_guard.last_block_hash
        {
            true
        } else {
            false
        }
    }

    pub fn election(&self, hash: &QualifiedRoot) -> Option<Arc<Election>> {
        let guard = self.mutex.lock().unwrap();
        guard.roots.get(hash).map(|i| i.election.clone())
    }

    pub fn votes_with_weight(&self, election: &Election) -> Vec<VoteWithWeightInfo> {
        let mut sorted_votes: BTreeMap<TallyKey, Vec<VoteWithWeightInfo>> = BTreeMap::new();
        let guard = election.mutex.lock().unwrap();
        for (&representative, info) in &guard.last_votes {
            if representative == HardenedConstants::get().not_an_account {
                continue;
            }
            let weight = self.ledger.weight(&representative);
            let vote_with_weight = VoteWithWeightInfo {
                representative,
                time: info.time,
                timestamp: info.timestamp,
                hash: info.hash,
                weight,
            };
            sorted_votes
                .entry(TallyKey(weight))
                .or_default()
                .push(vote_with_weight);
        }
        let result: Vec<_> = sorted_votes
            .values_mut()
            .map(|i| std::mem::take(i))
            .flatten()
            .collect();
        result
    }

    pub fn request_loop(&self) {
        let mut guard = self.mutex.lock().unwrap();
        while !guard.stopped {
            let stamp = Instant::now();
            self.stats.inc(StatType::Active, DetailType::Loop);
            guard = self.request_confirm(guard);
            guard = self.request_loop2(stamp, guard);
        }
    }

    pub fn request_confirm<'a>(
        &'a self,
        guard: MutexGuard<'a, ActiveElectionsState>,
    ) -> MutexGuard<'a, ActiveElectionsState> {
        let this_loop_target = guard.roots.len();
        let elections = Self::list_active_impl(this_loop_target, &guard);
        drop(guard);

        let mut solicitor = ConfirmationSolicitor::new(&self.network_params, &self.network);
        solicitor.prepare(&self.online_reps.lock().unwrap().peered_principal_reps());

        /*
         * Loop through active elections in descending order of proof-of-work difficulty, requesting confirmation
         *
         * Only up to a certain amount of elections are queued for confirmation request and block rebroadcasting. The remaining elections can still be confirmed if votes arrive
         * Elections extending the soft config.size limit are flushed after a certain time-to-live cutoff
         * Flushed elections are later re-activated via frontier confirmation
         */
        for election in elections {
            let confirmed = self.confirmed(&election);
            if confirmed || self.transition_time(&mut solicitor, &election) {
                self.erase(&election.qualified_root);
            }
        }

        solicitor.flush();
        self.mutex.lock().unwrap()
    }

    // Returns a list of elections sorted by difficulty
    pub fn list_active(&self, max: usize) -> Vec<Arc<Election>> {
        self.mutex
            .lock()
            .unwrap()
            .roots
            .iter_sequenced()
            .map(|i| i.election.clone())
            .take(max)
            .collect()
    }

    /// Returns a list of elections sorted by difficulty, mutex must be locked
    fn list_active_impl(
        max: usize,
        guard: &MutexGuard<ActiveElectionsState>,
    ) -> Vec<Arc<Election>> {
        guard
            .roots
            .iter_sequenced()
            .map(|i| i.election.clone())
            .take(max)
            .collect()
    }

    pub fn erase(&self, root: &QualifiedRoot) -> bool {
        let guard = self.mutex.lock().unwrap();
        if let Some(entry) = guard.roots.get(root) {
            let election = entry.election.clone();
            self.cleanup_election(guard, &election);
            true
        } else {
            false
        }
    }

    fn transition_time(
        &self,
        solicitor: &mut ConfirmationSolicitor,
        election: &Arc<Election>,
    ) -> bool {
        let mut guard = election.mutex.lock().unwrap();
        let mut result = false;
        match guard.state {
            ElectionState::Passive => {
                if self.base_latency() * Election::PASSIVE_DURATION_FACTOR
                    < election.election_start.elapsed()
                {
                    guard
                        .state_change(ElectionState::Passive, ElectionState::Active)
                        .unwrap();
                }
            }
            ElectionState::Active => {
                self.broadcast_vote(election, &mut guard);
                self.broadcast_block(solicitor, election, &mut guard);
                self.send_confirm_req(solicitor, election, &guard);
            }
            ElectionState::Confirmed => {
                result = true; // Return true to indicate this election should be cleaned up
                self.broadcast_block(solicitor, election, &mut guard); // Ensure election winner is broadcasted
                guard
                    .state_change(ElectionState::Confirmed, ElectionState::ExpiredConfirmed)
                    .unwrap();
            }
            ElectionState::ExpiredConfirmed | ElectionState::ExpiredUnconfirmed => {
                unreachable!()
            }
            ElectionState::Cancelled => {
                return true; // Clean up cancelled elections immediately
            }
        }

        if !guard.is_confirmed() && election.time_to_live() < election.election_start.elapsed() {
            // It is possible the election confirmed while acquiring the mutex
            // state_change returning true would indicate it
            let state = guard.state;
            if guard
                .state_change(state, ElectionState::ExpiredUnconfirmed)
                .is_ok()
            {
                trace!(qualified_root = ?election.qualified_root, "election expired");
                result = true; // Return true to indicate this election should be cleaned up
                guard.status.election_status_type = ElectionStatusType::Stopped;
            }
        }

        result
    }

    fn send_confirm_req(
        &self,
        solicitor: &mut ConfirmationSolicitor,
        election: &Election,
        election_guard: &MutexGuard<ElectionData>,
    ) {
        if self.confirm_req_time(election) < election.last_req_elapsed() {
            if !solicitor.add(election, election_guard) {
                election.set_last_req();
                election
                    .confirmation_request_count
                    .fetch_add(1, Ordering::SeqCst);
            }
        }
    }

    pub fn process_confirmed(&self, status: ElectionStatus, iteration: u64) {
        self.vote_applier.process_confirmed(status, iteration)
    }

    fn block_already_cemented_callback(&self, hash: &BlockHash) {
        // Depending on timing there is a situation where the election_winner_details is not reset.
        // This can happen when a block wins an election, and the block is confirmed + observer
        // called before the block hash gets added to election_winner_details. If the block is confirmed
        // callbacks have already been done, so we can safely just remove it.
        self.vote_applier.remove_election_winner_details(hash);
    }

    pub fn collect_container_info(&self, name: impl Into<String>) -> ContainerInfoComponent {
        let guard = self.mutex.lock().unwrap();
        ContainerInfoComponent::Composite(
            name.into(),
            vec![
                ContainerInfoComponent::Leaf(ContainerInfo {
                    name: "roots".to_string(),
                    count: guard.roots.len(),
                    sizeof_element: OrderedRoots::ELEMENT_SIZE,
                }),
                ContainerInfoComponent::Leaf(ContainerInfo {
                    name: "normal".to_string(),
                    count: guard.count_by_behavior(ElectionBehavior::Priority),
                    sizeof_element: 0,
                }),
                ContainerInfoComponent::Leaf(ContainerInfo {
                    name: "hinted".to_string(),
                    count: guard.count_by_behavior(ElectionBehavior::Hinted),
                    sizeof_element: 0,
                }),
                ContainerInfoComponent::Leaf(ContainerInfo {
                    name: "optimistic".to_string(),
                    count: guard.count_by_behavior(ElectionBehavior::Optimistic),
                    sizeof_element: 0,
                }),
                self.vote_applier.collect_container_info("vote_applier"),
                self.recently_confirmed
                    .collect_container_info("recently_confirmed"),
                ContainerInfoComponent::Composite(
                    "recently_cemented".to_string(),
                    vec![ContainerInfoComponent::Leaf(ContainerInfo {
                        name: "cemented".to_string(),
                        count: self.recently_cemented.lock().unwrap().len(),
                        sizeof_element: size_of::<ElectionStatus>(),
                    })],
                ),
            ],
        )
    }
}

impl Drop for ActiveElections {
    fn drop(&mut self) {
        // Thread must be stopped before destruction
        debug_assert!(self.thread.lock().unwrap().is_none());
    }
}

#[derive(PartialEq, Eq)]
pub struct TallyKey(pub Amount);

impl TallyKey {
    pub fn amount(&self) -> Amount {
        self.0.clone()
    }
}

impl Deref for TallyKey {
    type Target = Amount;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Ord for TallyKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.0.cmp(&self.0)
    }
}

impl PartialOrd for TallyKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        other.0.partial_cmp(&self.0)
    }
}

impl From<Amount> for TallyKey {
    fn from(value: Amount) -> Self {
        Self(value)
    }
}

pub struct ActiveElectionsState {
    roots: OrderedRoots,
    pub stopped: bool,
    pub manual_count: usize,
    pub priority_count: usize,
    pub hinted_count: usize,
    pub optimistic_count: usize,
}

impl ActiveElectionsState {
    pub fn count_by_behavior(&self, behavior: ElectionBehavior) -> usize {
        match behavior {
            ElectionBehavior::Manual => self.manual_count,
            ElectionBehavior::Priority => self.priority_count,
            ElectionBehavior::Hinted => self.hinted_count,
            ElectionBehavior::Optimistic => self.optimistic_count,
        }
    }

    pub fn count_by_behavior_mut(&mut self, behavior: ElectionBehavior) -> &mut usize {
        match behavior {
            ElectionBehavior::Manual => &mut self.manual_count,
            ElectionBehavior::Priority => &mut self.priority_count,
            ElectionBehavior::Hinted => &mut self.hinted_count,
            ElectionBehavior::Optimistic => &mut self.optimistic_count,
        }
    }
}

#[derive(Default)]
pub(crate) struct OrderedRoots {
    by_root: HashMap<QualifiedRoot, Entry>,
    sequenced: Vec<QualifiedRoot>,
}

impl OrderedRoots {
    pub const ELEMENT_SIZE: usize = size_of::<QualifiedRoot>() * 2 + size_of::<Arc<Election>>();

    pub fn insert(&mut self, entry: Entry) {
        let root = entry.root.clone();
        if self.by_root.insert(root.clone(), entry).is_none() {
            self.sequenced.push(root);
        }
    }

    pub fn get(&self, root: &QualifiedRoot) -> Option<&Entry> {
        self.by_root.get(root)
    }

    pub fn erase(&mut self, root: &QualifiedRoot) -> Option<Entry> {
        let erased = self.by_root.remove(root);
        if erased.is_some() {
            self.sequenced.retain(|x| x != root)
        }
        erased
    }

    pub fn clear(&mut self) {
        self.sequenced.clear();
        self.by_root.clear();
    }

    pub fn len(&self) -> usize {
        self.sequenced.len()
    }

    pub fn iter_sequenced(&self) -> impl Iterator<Item = &Entry> {
        self.sequenced.iter().map(|r| self.by_root.get(r).unwrap())
    }
}

pub trait ActiveElectionsExt {
    fn initialize(&self);
    fn start(&self);
    fn stop(&self);
    fn force_confirm(&self, election: &Arc<Election>);
    fn try_confirm(&self, election: &Arc<Election>, hash: &BlockHash);
    /// Distinguishes replay votes, cannot be determined if the block is not in any election
    fn block_cemented_callback(
        &self,
        tx: &LmdbReadTransaction,
        block: &BlockEnum,
        confirmation_root: &BlockHash,
    );
    fn publish_block(&self, block: &Arc<BlockEnum>) -> bool;
    fn insert(
        &self,
        block: &Arc<BlockEnum>,
        election_behavior: ElectionBehavior,
        erased_callback: Option<ErasedCallback>,
    ) -> (bool, Option<Arc<Election>>);
}

impl ActiveElectionsExt for Arc<ActiveElections> {
    fn initialize(&self) {
        let self_w = Arc::downgrade(self);
        self.confirming_set
            .add_batch_cemented_observer(Box::new(move |notification| {
                if let Some(active) = self_w.upgrade() {
                    {
                        let mut tx = active.ledger.read_txn();
                        for (block, confirmation_root) in &notification.cemented {
                            tx.refresh_if_needed();
                            active.block_cemented_callback(&tx, block, confirmation_root);
                        }
                    }
                    for hash in &notification.already_cemented {
                        active.block_already_cemented_callback(hash);
                    }
                }
            }));

        let self_w = Arc::downgrade(self);
        // Notify elections about alternative (forked) blocks
        self.block_processor
            .add_block_processed_observer(Box::new(move |status, context| {
                if matches!(status, BlockStatus::Fork) {
                    if let Some(active) = self_w.upgrade() {
                        active.publish_block(&context.block);
                    }
                }
            }));
    }

    fn start(&self) {
        if self.flags.disable_request_loop {
            return;
        }

        let mut guard = self.thread.lock().unwrap();
        let self_l = Arc::clone(self);
        assert!(guard.is_none());
        *guard = Some(
            std::thread::Builder::new()
                .name("Request loop".to_string())
                .spawn(Box::new(move || {
                    self_l.request_loop();
                }))
                .unwrap(),
        );
    }

    fn stop(&self) {
        self.mutex.lock().unwrap().stopped = true;
        self.condition.notify_all();
        if let Some(join_handle) = self.thread.lock().unwrap().take() {
            join_handle.join().unwrap();
        }
        self.clear();
    }

    fn force_confirm(&self, election: &Arc<Election>) {
        assert!(self.network_params.network.is_dev_network());
        let guard = election.mutex.lock().unwrap();
        self.vote_applier.confirm_once(guard, election);
    }

    fn try_confirm(&self, election: &Arc<Election>, hash: &BlockHash) {
        let guard = election.mutex.lock().unwrap();
        if let Some(winner) = &guard.status.winner {
            if winner.hash() == *hash {
                if !guard.is_confirmed() {
                    self.vote_applier.confirm_once(guard, election);
                }
            }
        }
    }

    fn block_cemented_callback(
        &self,
        tx: &LmdbReadTransaction,
        block: &BlockEnum,
        confirmation_root: &BlockHash,
    ) {
        if let Some(election) = self.election(&block.qualified_root()) {
            self.try_confirm(&election, &block.hash());
        }
        let votes: Vec<VoteWithWeightInfo>;
        let mut status: ElectionStatus;
        let election = self
            .vote_applier
            .remove_election_winner_details(&block.hash());
        if let Some(election) = &election {
            status = election.mutex.lock().unwrap().status.clone();
            votes = self.votes_with_weight(election);
        } else {
            status = ElectionStatus {
                winner: Some(Arc::new(block.clone())),
                ..Default::default()
            };
            votes = Vec::new();
        }
        if block.hash() == *confirmation_root {
            status.election_status_type = ElectionStatusType::ActiveConfirmedQuorum;
        } else if election.is_some() {
            status.election_status_type = ElectionStatusType::ActiveConfirmationHeight;
        } else {
            status.election_status_type = ElectionStatusType::InactiveConfirmationHeight;
        }

        self.recently_cemented
            .lock()
            .unwrap()
            .push_back(status.clone());

        self.stats
            .inc(StatType::ActiveElections, DetailType::Cemented);
        self.stats.inc(
            StatType::ActiveElectionsCemented,
            status.election_status_type.into(),
        );

        self.notify_observers(tx, &status, &votes);

        let cemented_bootstrap_count_reached =
            self.ledger.cemented_count() >= self.ledger.bootstrap_weight_max_blocks();
        let was_active = status.election_status_type == ElectionStatusType::ActiveConfirmedQuorum
            || status.election_status_type == ElectionStatusType::ActiveConfirmationHeight;

        // Next-block activations are only done for blocks with previously active elections
        if cemented_bootstrap_count_reached && was_active && !self.flags.disable_activate_successors
        {
            let guard = self.activate_successors.lock().unwrap();
            //TODO remove Arc
            (guard)(tx, &Arc::new(block.clone()));
        }
    }

    fn publish_block(&self, block: &Arc<BlockEnum>) -> bool {
        let mut guard = self.mutex.lock().unwrap();
        let root = block.qualified_root();
        let mut result = true;
        if let Some(entry) = guard.roots.get(&root) {
            let election = entry.election.clone();
            drop(guard);
            result = self.publish(block, &election);
            if !result {
                guard = self.mutex.lock().unwrap();
                self.vote_router
                    .connect(block.hash(), Arc::downgrade(&election));
                drop(guard);

                self.vote_cache_processor.trigger(block.hash());

                self.stats
                    .inc(StatType::Active, DetailType::ElectionBlockConflict);
                debug!("Block was added to an existing election: {}", block.hash());
            }
        }

        result
    }

    fn insert(
        &self,
        block: &Arc<BlockEnum>,
        election_behavior: ElectionBehavior,
        erased_callback: Option<ErasedCallback>,
    ) -> (bool, Option<Arc<Election>>) {
        let mut election_result = None;
        let mut inserted = false;

        let mut guard = self.mutex.lock().unwrap();

        if guard.stopped {
            return (false, None);
        }

        let root = block.qualified_root();
        let hash = block.hash();
        let existing = guard.roots.get(&root);

        if let Some(existing) = existing {
            election_result = Some(existing.election.clone());
        } else {
            if !self.recently_confirmed.root_exists(&root) {
                inserted = true;
                let online_reps = self.online_reps.clone();
                let relative_time = self.relative_time;
                let observer_rep_cb = Box::new(move |rep| {
                    // Representative is defined as online if replying to live votes or rep_crawler queries
                    online_reps
                        .lock()
                        .unwrap()
                        .vote_observed(rep, relative_time.elapsed());
                });

                let id = NEXT_ELECTION_ID.fetch_add(1, Ordering::Relaxed);
                let election = Arc::new(Election::new(
                    id,
                    Arc::clone(block),
                    election_behavior,
                    Box::new(|_| {}),
                    observer_rep_cb,
                ));
                guard.roots.insert(Entry {
                    root,
                    election: election.clone(),
                    erased_callback,
                });
                self.vote_router.connect(hash, Arc::downgrade(&election));

                // Keep track of election count by election type
                *guard.count_by_behavior_mut(election.behavior) += 1;

                self.stats
                    .inc(StatType::ActiveElections, DetailType::Started);
                self.stats
                    .inc(StatType::ActiveElectionsStarted, election_behavior.into());

                trace!(behavior = ?election_behavior, ?election, "active started");

                debug!(
                    "Started new election for block: {} (behavior: {:?})",
                    hash, election_behavior
                );

                election_result = Some(election);
            } else {
                // result is not set
            }
        }
        drop(guard);

        if inserted {
            debug_assert!(election_result.is_some());

            self.vote_cache_processor.trigger(hash);

            {
                let callbacks = self.active_started_observer.lock().unwrap();
                for callback in callbacks.iter() {
                    (callback)(hash);
                }
            }
            self.vacancy_update.lock().unwrap()();
        }

        // Votes are generated for inserted or ongoing elections
        if let Some(election) = &election_result {
            let mut guard = election.mutex.lock().unwrap();
            self.broadcast_vote(election, &mut guard);
        }

        (inserted, election_result)
    }
}

pub(crate) struct ActiveElectionsInfo {
    pub total: usize,
    pub priority: usize,
    pub hinted: usize,
    pub optimistic: usize,
}

pub(crate) struct Entry {
    root: QualifiedRoot,
    election: Arc<Election>,
    erased_callback: Option<ErasedCallback>,
}

pub(crate) type ErasedCallback = Box<dyn Fn(&Arc<Election>) + Send + Sync>;
