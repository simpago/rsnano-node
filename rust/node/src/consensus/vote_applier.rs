use crate::{
    block_processing::BlockProcessor,
    cementation::ConfirmingSet,
    config::NodeConfig,
    consensus::{ElectionState, VoteInfo},
    representatives::OnlineReps,
    stats::{DetailType, StatType, Stats},
    utils::ThreadPool,
    wallets::Wallets,
    NetworkParams,
};

use super::{
    election_schedulers::ElectionSchedulers, Election, ElectionData, ElectionStatus,
    LocalVoteHistory, RecentlyConfirmedCache, TallyKey, VoteGenerators,
};
use rsnano_core::{
    utils::{ContainerInfo, ContainerInfoComponent},
    Amount, BlockEnum, BlockHash, PublicKey, VoteCode, VoteSource,
};
use rsnano_ledger::Ledger;
use std::{
    collections::{BTreeMap, HashMap},
    mem::size_of,
    sync::{atomic::Ordering, Arc, Mutex, MutexGuard, RwLock, Weak},
    time::{Duration, SystemTime},
};
use tracing::trace;

pub struct VoteApplier {
    ledger: Arc<Ledger>,
    network_params: NetworkParams,
    online_reps: Arc<Mutex<OnlineReps>>,
    stats: Arc<Stats>,
    vote_generators: Arc<VoteGenerators>,
    block_processor: Arc<BlockProcessor>,
    node_config: NodeConfig,
    history: Arc<LocalVoteHistory>,
    wallets: Arc<Wallets>,
    recently_confirmed: Arc<RecentlyConfirmedCache>,
    confirming_set: Arc<ConfirmingSet>,
    workers: Arc<dyn ThreadPool>,
    election_winner_details: Mutex<HashMap<BlockHash, Arc<Election>>>,
    election_schedulers: RwLock<Option<Weak<ElectionSchedulers>>>,
}

impl VoteApplier {
    pub(crate) fn new(
        ledger: Arc<Ledger>,
        network_params: NetworkParams,
        online_reps: Arc<Mutex<OnlineReps>>,
        stats: Arc<Stats>,
        vote_generators: Arc<VoteGenerators>,
        block_processor: Arc<BlockProcessor>,
        node_config: NodeConfig,
        history: Arc<LocalVoteHistory>,
        wallets: Arc<Wallets>,
        recently_confirmed: Arc<RecentlyConfirmedCache>,
        confirming_set: Arc<ConfirmingSet>,
        workers: Arc<dyn ThreadPool>,
    ) -> Self {
        Self {
            ledger,
            network_params,
            online_reps,
            stats,
            vote_generators,
            block_processor,
            node_config,
            history,
            wallets,
            recently_confirmed,
            confirming_set,
            workers,
            election_winner_details: Mutex::new(HashMap::new()),
            election_schedulers: RwLock::new(None),
        }
    }

    pub(crate) fn set_election_schedulers(&self, schedulers: &Arc<ElectionSchedulers>) {
        *self.election_schedulers.write().unwrap() = Some(Arc::downgrade(&schedulers));
    }

    /// Calculates minimum time delay between subsequent votes when processing non-final votes
    pub fn cooldown_time(&self, weight: Amount) -> Duration {
        let online_stake = {
            self.online_reps
                .lock()
                .unwrap()
                .trended_weight_or_minimum_online_weight()
        };
        if weight > online_stake / 20 {
            // Reps with more than 5% weight
            Duration::from_secs(1)
        } else if weight > online_stake / 100 {
            // Reps with more than 1% weight
            Duration::from_secs(5)
        } else {
            // The rest of smaller reps
            Duration::from_secs(15)
        }
    }

    pub fn tally_impl(
        &self,
        guard: &mut MutexGuard<ElectionData>,
    ) -> BTreeMap<TallyKey, Arc<BlockEnum>> {
        let mut block_weights: HashMap<BlockHash, Amount> = HashMap::new();
        let mut final_weights: HashMap<BlockHash, Amount> = HashMap::new();
        for (account, info) in &guard.last_votes {
            let rep_weight = self.ledger.weight(account);
            *block_weights.entry(info.hash).or_default() += rep_weight;
            if info.timestamp == u64::MAX {
                *final_weights.entry(info.hash).or_default() += rep_weight;
            }
        }
        guard.last_tally.clear();
        for (&hash, &weight) in &block_weights {
            guard.last_tally.insert(hash, weight);
        }
        let mut result = BTreeMap::new();
        for (hash, weight) in &block_weights {
            if let Some(block) = guard.last_blocks.get(hash) {
                result.insert(TallyKey(*weight), Arc::clone(block));
            }
        }
        // Calculate final votes sum for winner
        if !final_weights.is_empty() && !result.is_empty() {
            let winner_hash = result.first_key_value().unwrap().1.hash();
            if let Some(final_weight) = final_weights.get(&winner_hash) {
                guard.final_weight = *final_weight;
            }
        }
        result
    }

    pub fn remove_votes(
        &self,
        election: &Election,
        guard: &mut MutexGuard<ElectionData>,
        hash: &BlockHash,
    ) {
        if self.node_config.enable_voting && self.wallets.voting_reps_count() > 0 {
            // Remove votes from election
            let list_generated_votes = self.history.votes(&election.root, hash, false);
            for vote in list_generated_votes {
                guard.last_votes.remove(&vote.voting_account);
            }
            // Clear votes cache
            self.history.erase(&election.root);
        }
    }

    pub fn have_quorum(&self, tally: &BTreeMap<TallyKey, Arc<BlockEnum>>) -> bool {
        let mut it = tally.keys();
        let first = it.next().map(|i| i.amount()).unwrap_or_default();
        let second = it.next().map(|i| i.amount()).unwrap_or_default();
        let delta = self.online_reps.lock().unwrap().quorum_delta();
        first - second >= delta
    }

    pub fn add_election_winner_details(&self, hash: BlockHash, election: Arc<Election>) {
        self.election_winner_details
            .lock()
            .unwrap()
            .insert(hash, election);
    }

    pub fn election_winner_details_len(&self) -> usize {
        self.election_winner_details.lock().unwrap().len()
    }

    pub fn remove_election_winner_details(&self, hash: &BlockHash) -> Option<Arc<Election>> {
        let election = {
            let mut guard = self.election_winner_details.lock().unwrap();
            guard.remove(hash)
        };

        self.vacancy_changed();

        election
    }

    fn vacancy_changed(&self) {
        let schedulers = self.election_schedulers.read().unwrap();
        if let Some(schedulers) = &*schedulers {
            if let Some(schedulers) = schedulers.upgrade() {
                schedulers.notify();
            }
        }
    }

    pub fn collect_container_info(&self, name: impl Into<String>) -> ContainerInfoComponent {
        ContainerInfoComponent::Composite(
            name.into(),
            vec![ContainerInfoComponent::Leaf(ContainerInfo {
                name: "election_winner_details".to_string(),
                count: self.election_winner_details.lock().unwrap().len(),
                sizeof_element: size_of::<BlockHash>() + size_of::<Arc<Election>>(),
            })],
        )
    }
}

pub trait VoteApplierExt {
    fn vote(
        &self,
        election: &Arc<Election>,
        rep: &PublicKey,
        timestamp: u64,
        block_hash: &BlockHash,
        vote_source: VoteSource,
    ) -> VoteCode;
    fn confirm_if_quorum(&self, election_lock: MutexGuard<ElectionData>, election: &Arc<Election>);
    fn confirm_once(&self, election_lock: MutexGuard<ElectionData>, election: &Arc<Election>);
    fn process_confirmed(&self, status: ElectionStatus, iteration: u64);
}

impl VoteApplierExt for Arc<VoteApplier> {
    fn vote(
        &self,
        election: &Arc<Election>,
        rep: &PublicKey,
        timestamp: u64,
        block_hash: &BlockHash,
        vote_source: VoteSource,
    ) -> VoteCode {
        let weight = self.ledger.weight(rep);
        if !self.network_params.network.is_dev_network()
            && weight <= self.online_reps.lock().unwrap().minimum_principal_weight()
        {
            return VoteCode::Indeterminate;
        }

        let mut guard = election.mutex.lock().unwrap();

        if let Some(last_vote) = guard.last_votes.get(rep) {
            if last_vote.timestamp > timestamp {
                return VoteCode::Replay;
            }
            if last_vote.timestamp == timestamp && !(last_vote.hash < *block_hash) {
                return VoteCode::Replay;
            }

            let max_vote = timestamp == u64::MAX && last_vote.timestamp < timestamp;

            let mut past_cooldown = true;
            // Only cooldown live votes
            if vote_source != VoteSource::Cache {
                let cooldown = self.cooldown_time(weight);
                past_cooldown = last_vote.time <= SystemTime::now() - cooldown;
            }

            if !max_vote && !past_cooldown {
                return VoteCode::Ignored;
            }
        }
        guard
            .last_votes
            .insert(*rep, VoteInfo::new(timestamp, *block_hash));

        if vote_source != VoteSource::Cache {
            (election.live_vote_action)(*rep);
        }

        self.stats.inc(StatType::Election, DetailType::Vote);
        self.stats.inc(StatType::ElectionVote, vote_source.into());
        tracing::trace!(
            qualified_root = ?election.qualified_root,
            account = %rep,
            hash = %block_hash,
            timestamp,
            ?vote_source,
            ?weight,
            "vote processed");

        if !guard.is_confirmed() {
            self.confirm_if_quorum(guard, election);
        }
        VoteCode::Vote
    }

    fn confirm_if_quorum(
        &self,
        mut election_lock: MutexGuard<ElectionData>,
        election: &Arc<Election>,
    ) {
        let tally = self.tally_impl(&mut election_lock);
        let (amount, block) = tally.first_key_value().unwrap();
        let winner_hash = block.hash();
        election_lock.status.tally = amount.amount();
        election_lock.status.final_tally = election_lock.final_weight;
        let status_winner_hash = election_lock.status.winner.as_ref().unwrap().hash();
        let mut sum = Amount::zero();
        for k in tally.keys() {
            sum += k.amount();
        }
        if sum >= self.online_reps.lock().unwrap().quorum_delta()
            && winner_hash != status_winner_hash
        {
            election_lock.status.winner = Some(Arc::clone(block));
            self.remove_votes(election, &mut election_lock, &status_winner_hash);
            self.block_processor.force(Arc::clone(block));
        }

        if self.have_quorum(&tally) {
            if !election.is_quorum.swap(true, Ordering::SeqCst)
                && self.node_config.enable_voting
                && self.wallets.voting_reps_count() > 0
            {
                self.vote_generators
                    .generate_final_vote(&election.root, &status_winner_hash);
            }
            let quorum_delta = self.online_reps.lock().unwrap().quorum_delta();
            if election_lock.final_weight >= quorum_delta {
                self.confirm_once(election_lock, election);
            }
        }
    }

    fn confirm_once(&self, mut election_lock: MutexGuard<ElectionData>, election: &Arc<Election>) {
        // This must be kept above the setting of election state, as dependent confirmed elections require up to date changes to election_winner_details
        let mut winners_guard = self.election_winner_details.lock().unwrap();
        let mut status = election_lock.status.clone();
        let old_state = election_lock.state;
        let just_confirmed = old_state != ElectionState::Confirmed;
        election_lock.state = ElectionState::Confirmed;
        if just_confirmed && !winners_guard.contains_key(&status.winner.as_ref().unwrap().hash()) {
            winners_guard.insert(status.winner.as_ref().unwrap().hash(), Arc::clone(election));
            drop(winners_guard);

            election_lock.update_status_to_confirmed(election);
            status = election_lock.status.clone();

            self.recently_confirmed.put(
                election.qualified_root.clone(),
                status.winner.as_ref().unwrap().hash(),
            );

            trace!(
                qualified_root = ?election.qualified_root,
                "election confirmed"
            );
            drop(election_lock);

            let self_l = Arc::clone(&self);
            let election = Arc::clone(election);
            self.workers.push_task(Box::new(move || {
                let block = Arc::clone(status.winner.as_ref().unwrap());
                self_l.process_confirmed(status, 0);
                (election.confirmation_action)(block);
            }));
        }
    }

    fn process_confirmed(&self, status: ElectionStatus, mut iteration: u64) {
        let hash = status.winner.as_ref().unwrap().hash();
        let num_iters = (self.node_config.block_processor_batch_max_time_ms
            / self.network_params.node.process_confirmed_interval_ms)
            as u64
            * 4;
        let block = {
            let tx = self.ledger.read_txn();
            self.ledger.any().get_block(&tx, &hash)
        };
        if let Some(block) = block {
            trace!(block = ?block,"process confirmed");
            self.confirming_set.add(block.hash());
        } else if iteration < num_iters {
            iteration += 1;
            let self_w = Arc::downgrade(self);
            self.workers.add_delayed_task(
                Duration::from_millis(
                    self.network_params.node.process_confirmed_interval_ms as u64,
                ),
                Box::new(move || {
                    if let Some(self_l) = self_w.upgrade() {
                        self_l.process_confirmed(status, iteration);
                    }
                }),
            );
        } else {
            // Do some cleanup due to this block never being processed by confirmation height processor
            self.remove_election_winner_details(&hash);
        }
    }
}
