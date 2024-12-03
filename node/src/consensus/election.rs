use super::ElectionStatus;
use crate::{
    stats::{DetailType, StatType},
    utils::HardenedConstants,
};
use rsnano_core::{
    Amount, Block, BlockHash, PublicKey, QualifiedRoot, Root, SavedBlock, SavedOrUnsavedBlock,
};
use std::{
    collections::HashMap,
    fmt::Debug,
    sync::{
        atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering},
        Mutex, RwLock,
    },
    time::{Duration, Instant, SystemTime},
};

pub static NEXT_ELECTION_ID: AtomicUsize = AtomicUsize::new(1);

//TODO remove the many RwLocks
pub struct Election {
    pub id: usize,
    pub mutex: Mutex<ElectionData>,
    pub root: Root,
    pub qualified_root: QualifiedRoot,
    pub is_quorum: AtomicBool,
    pub confirmation_request_count: AtomicU32,
    // These are modified while not holding the mutex from transition_time only
    last_block: RwLock<Instant>,
    pub last_req: RwLock<Option<Instant>>,
    pub behavior: ElectionBehavior,
    pub election_start: Instant,
    pub confirmation_action: Box<dyn Fn(Block) + Send + Sync>,
    pub live_vote_action: Box<dyn Fn(PublicKey) + Send + Sync>,
    height: u64,
}

impl Election {
    pub const PASSIVE_DURATION_FACTOR: u32 = 5;

    pub fn new(
        id: usize,
        block: SavedBlock,
        behavior: ElectionBehavior,
        confirmation_action: Box<dyn Fn(Block) + Send + Sync>,
        live_vote_action: Box<dyn Fn(PublicKey) + Send + Sync>,
    ) -> Self {
        let root = block.root();
        let qualified_root = block.qualified_root();
        let height = block.height();

        let data = ElectionData {
            status: ElectionStatus {
                winner: Some(rsnano_core::SavedOrUnsavedBlock::Saved(block.clone())),
                election_end: SystemTime::now(),
                block_count: 1,
                election_status_type: super::ElectionStatusType::Ongoing,
                ..Default::default()
            },
            last_votes: HashMap::from([(
                HardenedConstants::get().not_an_account_key,
                VoteInfo::new(0, block.hash()),
            )]),
            last_blocks: HashMap::from([(block.hash(), SavedOrUnsavedBlock::Saved(block))]),
            state: ElectionState::Passive,
            state_start: Instant::now(),
            last_tally: HashMap::new(),
            final_weight: Amount::zero(),
            last_vote: None,
            last_block_hash: BlockHash::zero(),
        };

        Self {
            id,
            mutex: Mutex::new(data),
            root,
            qualified_root,
            is_quorum: AtomicBool::new(false),
            confirmation_request_count: AtomicU32::new(0),
            last_block: RwLock::new(Instant::now()),
            behavior,
            election_start: Instant::now(),
            last_req: RwLock::new(None),
            confirmation_action,
            live_vote_action,
            height,
        }
    }

    pub fn duration(&self) -> Duration {
        self.election_start.elapsed()
    }

    pub fn state(&self) -> ElectionState {
        self.mutex.lock().unwrap().state
    }

    pub fn transition_active(&self) {
        let _ = self
            .mutex
            .lock()
            .unwrap()
            .state_change(ElectionState::Passive, ElectionState::Active);
    }

    pub fn cancel(&self) {
        let mut guard = self.mutex.lock().unwrap();
        let current = guard.state;
        let _ = guard.state_change(current, ElectionState::Cancelled);
    }

    pub fn set_last_req(&self) {
        *self.last_req.write().unwrap() = Some(Instant::now());
    }

    pub fn last_req_elapsed(&self) -> Duration {
        match self.last_req.read().unwrap().as_ref() {
            Some(i) => i.elapsed(),
            None => Duration::from_secs(60 * 60 * 24 * 365), // Duration::MAX caused problems with C++
        }
    }

    pub fn set_last_block(&self) {
        *self.last_block.write().unwrap() = Instant::now();
    }

    pub fn last_block_elapsed(&self) -> Duration {
        self.last_block.read().unwrap().elapsed()
    }

    pub fn age(&self) -> Duration {
        self.mutex.lock().unwrap().state_start.elapsed()
    }

    pub fn failed(&self) -> bool {
        self.mutex.lock().unwrap().state == ElectionState::ExpiredUnconfirmed
    }

    pub fn time_to_live(&self) -> Duration {
        match self.behavior {
            ElectionBehavior::Manual | ElectionBehavior::Priority => Duration::from_secs(60 * 5),
            ElectionBehavior::Hinted | ElectionBehavior::Optimistic => Duration::from_secs(30),
        }
    }

    pub fn contains(&self, hash: &BlockHash) -> bool {
        self.mutex.lock().unwrap().last_blocks.contains_key(hash)
    }

    pub fn vote_count(&self) -> usize {
        self.mutex.lock().unwrap().last_votes.len()
    }

    pub fn winner_hash(&self) -> Option<BlockHash> {
        self.mutex
            .lock()
            .unwrap()
            .status
            .winner
            .as_ref()
            .map(|w| w.hash())
    }
}

impl Debug for Election {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Election")
            .field("id", &self.id)
            .field("qualified_root", &self.qualified_root)
            .field("behavior", &self.behavior)
            .field("height", &self.height)
            .finish()
    }
}

pub struct ElectionData {
    pub status: ElectionStatus,
    pub state: ElectionState,
    pub state_start: Instant,
    pub last_blocks: HashMap<BlockHash, SavedOrUnsavedBlock>,
    pub last_votes: HashMap<PublicKey, VoteInfo>,
    pub final_weight: Amount,
    pub last_tally: HashMap<BlockHash, Amount>,
    /** The last time vote for this election was generated */
    pub last_vote: Option<Instant>,
    pub last_block_hash: BlockHash,
}

impl ElectionData {
    pub fn is_confirmed(&self) -> bool {
        matches!(
            self.state,
            ElectionState::Confirmed | ElectionState::ExpiredConfirmed
        )
    }

    pub fn update_status_to_confirmed(&mut self, election: &Election) {
        self.status.election_end = SystemTime::now();
        self.status.election_duration = election.election_start.elapsed();
        self.status.confirmation_request_count =
            election.confirmation_request_count.load(Ordering::SeqCst);
        self.status.block_count = self.last_blocks.len() as u32;
        self.status.voter_count = self.last_votes.len() as u32;
    }

    pub fn state_change(
        &mut self,
        expected: ElectionState,
        desired: ElectionState,
    ) -> Result<(), ()> {
        if Self::valid_change(expected, desired) {
            if self.state == expected {
                self.state = desired;
                self.state_start = Instant::now();
                return Ok(());
            }
        }

        Err(())
    }

    fn valid_change(expected: ElectionState, desired: ElectionState) -> bool {
        match expected {
            ElectionState::Passive => matches!(
                desired,
                ElectionState::Active
                    | ElectionState::Confirmed
                    | ElectionState::ExpiredUnconfirmed
                    | ElectionState::Cancelled
            ),
            ElectionState::Active => matches!(
                desired,
                ElectionState::Confirmed
                    | ElectionState::ExpiredUnconfirmed
                    | ElectionState::Cancelled
            ),
            ElectionState::Confirmed => matches!(desired, ElectionState::ExpiredConfirmed),
            ElectionState::Cancelled
            | ElectionState::ExpiredConfirmed
            | ElectionState::ExpiredUnconfirmed => false,
        }
    }

    pub fn set_last_vote(&mut self) {
        self.last_vote = Some(Instant::now());
    }

    pub fn last_vote_elapsed(&self) -> Duration {
        match &self.last_vote {
            Some(i) => i.elapsed(),
            None => Duration::from_secs(60 * 60 * 24 * 365), // Duration::MAX caused problems with C++
        }
    }
}

#[derive(Clone)]
pub struct VoteInfo {
    pub time: SystemTime, // TODO use Instant
    pub timestamp: u64,
    pub hash: BlockHash,
}

impl VoteInfo {
    pub fn new(timestamp: u64, hash: BlockHash) -> Self {
        Self {
            time: SystemTime::now(),
            timestamp,
            hash,
        }
    }
}

impl Default for VoteInfo {
    fn default() -> Self {
        Self::new(0, BlockHash::zero())
    }
}

#[derive(FromPrimitive, Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ElectionState {
    Passive,   // only listening for incoming votes
    Active,    // actively request confirmations
    Confirmed, // confirmed but still listening for votes
    ExpiredConfirmed,
    ExpiredUnconfirmed,
    Cancelled,
}

impl ElectionState {
    pub fn is_confirmed(&self) -> bool {
        matches!(self, Self::Confirmed | Self::ExpiredConfirmed)
    }
}

impl From<ElectionState> for StatType {
    fn from(value: ElectionState) -> Self {
        match value {
            ElectionState::Passive | ElectionState::Active => StatType::ActiveElectionsDropped,
            ElectionState::Confirmed | ElectionState::ExpiredConfirmed => {
                StatType::ActiveElectionsConfirmed
            }
            ElectionState::ExpiredUnconfirmed => StatType::ActiveElectionsTimeout,
            ElectionState::Cancelled => StatType::ActiveElectionsCancelled,
        }
    }
}

impl From<ElectionState> for DetailType {
    fn from(value: ElectionState) -> Self {
        match value {
            ElectionState::Passive => DetailType::Passive,
            ElectionState::Active => DetailType::Active,
            ElectionState::Confirmed => DetailType::Confirmed,
            ElectionState::ExpiredConfirmed => DetailType::ExpiredConfirmed,
            ElectionState::ExpiredUnconfirmed => DetailType::ExpiredUnconfirmed,
            ElectionState::Cancelled => DetailType::Cancelled,
        }
    }
}

#[derive(FromPrimitive, Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ElectionBehavior {
    Manual,
    Priority,
    /**
     * Hinted elections:
     * - shorter timespan
     * - limited space inside AEC
     */
    Hinted,
    /**
     * Optimistic elections:
     * - shorter timespan
     * - limited space inside AEC
     * - more frequent confirmation requests
     */
    Optimistic,
}

impl From<ElectionBehavior> for DetailType {
    fn from(value: ElectionBehavior) -> Self {
        match value {
            ElectionBehavior::Manual => DetailType::Manual,
            ElectionBehavior::Priority => DetailType::Priority,
            ElectionBehavior::Hinted => DetailType::Hinted,
            ElectionBehavior::Optimistic => DetailType::Optimistic,
        }
    }
}
