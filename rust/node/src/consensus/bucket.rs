use super::{ActiveElections, Election, ElectionBehavior};
use crate::{
    consensus::ActiveElectionsExt,
    stats::{DetailType, StatType, Stats},
};
use rsnano_core::{utils::TomlWriter, Amount, BlockEnum, QualifiedRoot};
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::{Arc, Mutex},
};

#[derive(Clone, Deserialize, Serialize)]
pub struct PriorityBucketConfig {
    /// Maximum number of blocks to sort by priority per bucket.
    pub max_blocks: usize,

    /// Number of guaranteed slots per bucket available for election activation.
    pub reserved_elections: usize,

    /// Maximum number of slots per bucket available for election activation if the active election count is below the configured limit. (node.active_elections.size)
    pub max_elections: usize,
}

impl Default for PriorityBucketConfig {
    fn default() -> Self {
        Self {
            max_blocks: 1024 * 8,
            reserved_elections: 100,
            max_elections: 150,
        }
    }
}

impl PriorityBucketConfig {
    pub(crate) fn serialize_toml(&self, toml: &mut dyn TomlWriter) -> anyhow::Result<()> {
        toml.put_usize(
            "max_blocks",
            self.max_blocks,
            "Maximum number of blocks to sort by priority per bucket. \nType: uint64",
        )?;
        toml.put_usize ("reserved_elections", self.reserved_elections, "Number of guaranteed slots per bucket available for election activation. \nType: uint64")?;
        toml.put_usize ("max_elections", self.max_elections, "Maximum number of slots per bucket available for election activation if the active election count is below the configured limit. \nType: uint64")
    }
}

type Priority = u64;

/// A struct which holds an ordered set of blocks to be scheduled, ordered by their block arrival time
/// TODO: This combines both block ordering and election management, which makes the class harder to test. The functionality should be split.
pub struct Bucket {
    minimum_balance: Amount,
    config: PriorityBucketConfig,
    active: Arc<ActiveElections>,
    stats: Arc<Stats>,
    data: Mutex<BucketData>,
}

impl Bucket {
    pub fn new(
        minimum_balance: Amount,
        config: PriorityBucketConfig,
        active: Arc<ActiveElections>,
        stats: Arc<Stats>,
    ) -> Self {
        Self {
            minimum_balance,
            config,
            active,
            stats: stats.clone(),
            data: Mutex::new(BucketData {
                queue: BTreeSet::new(),
                elections: OrderedElections::default(),
                stats,
            }),
        }
    }

    pub fn can_accept(&self, priority: Amount) -> bool {
        priority >= self.minimum_balance
    }

    pub fn available(&self) -> bool {
        let guard = self.data.lock().unwrap();
        if let Some(first) = guard.queue.first() {
            self.election_vacancy(first.time, &guard)
        } else {
            false
        }
    }

    fn election_vacancy(&self, candidate: Priority, data: &BucketData) -> bool {
        let election_count = data.elections.len();
        if election_count < self.config.reserved_elections {
            true
        } else if election_count < self.config.max_elections {
            self.active.vacancy(ElectionBehavior::Priority) > 0
        } else if election_count > 0 {
            let lowest = data.elections.lowest_priority();

            // Compare to equal to drain duplicates
            if candidate <= lowest {
                // Bound number of reprioritizations
                election_count < self.config.max_elections * 2
            } else {
                false
            }
        } else {
            false
        }
    }

    fn election_overfill(&self, data: &BucketData) -> bool {
        if data.elections.len() < self.config.reserved_elections {
            false
        } else if data.elections.len() < self.config.max_elections {
            self.active.vacancy(ElectionBehavior::Priority) < 0
        } else {
            true
        }
    }

    pub fn update(&self) {
        let guard = self.data.lock().unwrap();
        if self.election_overfill(&guard) {
            guard.cancel_lowest_election();
        }
    }

    pub fn push(&self, time: u64, block: Arc<BlockEnum>) -> bool {
        let hash = block.hash();
        let mut guard = self.data.lock().unwrap();
        let inserted = guard.queue.insert(BlockEntry { time, block });
        if guard.queue.len() > self.config.max_blocks {
            if let Some(removed) = guard.queue.pop_last() {
                inserted && !(removed.time == time && removed.block.hash() == hash)
            } else {
                inserted
            }
        } else {
            inserted
        }
    }

    pub fn len(&self) -> usize {
        self.data.lock().unwrap().queue.len()
    }

    pub fn election_count(&self) -> usize {
        self.data.lock().unwrap().elections.len()
    }

    pub fn blocks(&self) -> Vec<Arc<BlockEnum>> {
        let guard = self.data.lock().unwrap();
        guard.queue.iter().map(|i| i.block.clone()).collect()
    }
}

pub(crate) trait BucketExt {
    fn activate(&self) -> bool;
}

impl BucketExt for Arc<Bucket> {
    fn activate(&self) -> bool {
        let mut guard = self.data.lock().unwrap();

        let Some(top) = guard.queue.pop_first() else {
            return false; // Not activated;
        };

        let block = top.block;
        let priority = top.time;

        let self_w = Arc::downgrade(self);
        let erase_callback = Box::new(move |election: &Arc<Election>| {
            let Some(self_l) = self_w.upgrade() else {
                return;
            };
            let mut guard = self_l.data.lock().unwrap();
            guard.elections.erase(&election.qualified_root);
        });

        let (inserted, election) =
            self.active
                .insert(&block, ElectionBehavior::Priority, Some(erase_callback));

        if inserted {
            let election = election.unwrap();
            guard.elections.insert(ElectionEntry {
                root: election.qualified_root.clone(),
                election,
                priority,
            });
            self.stats
                .inc(StatType::ElectionBucket, DetailType::ActivateSuccess);
        } else {
            self.stats
                .inc(StatType::ElectionBucket, DetailType::ActivateFailed);
        }

        inserted
    }
}

struct BucketData {
    queue: BTreeSet<BlockEntry>,
    elections: OrderedElections,
    stats: Arc<Stats>,
}

impl BucketData {
    fn cancel_lowest_election(&self) {
        if let Some(entry) = self.elections.entry_with_lowest_priority() {
            entry.election.cancel();
            self.stats
                .inc(StatType::ElectionBucket, DetailType::CancelLowest);
        }
    }
}

struct BlockEntry {
    time: u64,
    block: Arc<BlockEnum>,
}

impl Ord for BlockEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        let time_order = self.time.cmp(&other.time);
        match time_order {
            Ordering::Equal => self.block.hash().cmp(&other.block.hash()),
            _ => time_order,
        }
    }
}

impl PartialOrd for BlockEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for BlockEntry {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time && self.block.hash() == other.block.hash()
    }
}

impl Eq for BlockEntry {}

struct ElectionEntry {
    election: Arc<Election>,
    root: QualifiedRoot,
    priority: Priority,
}

#[derive(Default)]
struct OrderedElections {
    by_root: HashMap<QualifiedRoot, ElectionEntry>,
    sequenced: Vec<QualifiedRoot>,
    by_priority: BTreeMap<Priority, Vec<QualifiedRoot>>,
}

impl OrderedElections {
    fn insert(&mut self, entry: ElectionEntry) {
        let root = entry.root.clone();
        let priority = entry.priority;
        let old = self.by_root.insert(root.clone(), entry);
        if let Some(old) = old {
            self.erase_indices(old);
        }
        self.sequenced.push(root.clone());
        self.by_priority.entry(priority).or_default().push(root);
    }

    fn entry_with_lowest_priority(&self) -> Option<&ElectionEntry> {
        self.by_priority
            .first_key_value()
            .and_then(|(_, roots)| self.by_root.get(&roots[0]))
    }

    fn lowest_priority(&self) -> u64 {
        self.by_priority
            .first_key_value()
            .map(|(prio, _)| *prio)
            .unwrap_or_default()
    }

    fn len(&self) -> usize {
        self.sequenced.len()
    }

    fn erase(&mut self, root: &QualifiedRoot) {
        if let Some(entry) = self.by_root.remove(root) {
            self.erase_indices(entry)
        }
    }

    fn erase_indices(&mut self, entry: ElectionEntry) {
        let keys = self.by_priority.get_mut(&entry.priority).unwrap();
        if keys.len() == 1 {
            self.by_priority.remove(&entry.priority);
        } else {
            keys.retain(|i| *i != entry.root);
        }
        self.sequenced.retain(|i| *i != entry.root);
    }
}
