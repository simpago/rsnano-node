use super::{
    ordered_blocks::{BlockEntry, OrderedBlocks},
    ActiveElections, Election, ElectionBehavior,
};
use crate::{
    consensus::ActiveElectionsExt,
    stats::{DetailType, StatType, Stats},
};
use rsnano_core::{utils::UnixTimestamp, Amount, Block, BlockHash, QualifiedRoot, SavedBlock};
use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, Mutex},
};

#[derive(Clone, Debug, PartialEq)]
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
                queue: Default::default(),
                elections: OrderedElections::default(),
            }),
        }
    }

    pub fn can_accept(&self, priority: Amount) -> bool {
        priority >= self.minimum_balance
    }

    pub fn contains(&self, hash: &BlockHash) -> bool {
        self.data.lock().unwrap().queue.contains(hash)
    }

    pub fn available(&self) -> bool {
        let candidate: UnixTimestamp;
        let election_count: usize;
        let lowest: UnixTimestamp;

        {
            let guard = self.data.lock().unwrap();
            let Some(first) = guard.queue.first() else {
                return false;
            };

            candidate = first.time;
            election_count = guard.elections.len();
            lowest = guard.elections.lowest_priority();
        }

        if election_count < self.config.reserved_elections
            || election_count < self.config.max_elections
        {
            self.active.vacancy(ElectionBehavior::Priority) > 0
        } else if election_count > 0 {
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
            drop(guard);
            self.stats
                .inc(StatType::ElectionBucket, DetailType::CancelLowest);
        }
    }

    pub fn push(&self, time: UnixTimestamp, block: SavedBlock) -> bool {
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

    pub fn blocks(&self) -> Vec<Block> {
        let guard = self.data.lock().unwrap();
        guard.queue.iter().map(|i| i.block.clone().into()).collect()
    }
}

pub(crate) trait BucketExt {
    fn activate(&self) -> bool;
}

impl BucketExt for Arc<Bucket> {
    fn activate(&self) -> bool {
        let block: SavedBlock;
        let priority: UnixTimestamp;

        {
            let mut guard = self.data.lock().unwrap();

            let Some(top) = guard.queue.pop_first() else {
                return false; // Not activated;
            };

            block = top.block;
            priority = top.time;
        }

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
                .insert(block, ElectionBehavior::Priority, Some(erase_callback));

        if inserted {
            let election = election.unwrap();
            self.data.lock().unwrap().elections.insert(ElectionEntry {
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
    queue: OrderedBlocks,
    elections: OrderedElections,
}

impl BucketData {
    fn cancel_lowest_election(&self) {
        if let Some(entry) = self.elections.entry_with_lowest_priority() {
            entry.election.cancel();
        }
    }
}

struct ElectionEntry {
    election: Arc<Election>,
    root: QualifiedRoot,
    priority: UnixTimestamp,
}

#[derive(Default)]
struct OrderedElections {
    by_root: HashMap<QualifiedRoot, ElectionEntry>,
    sequenced: Vec<QualifiedRoot>,
    by_priority: BTreeMap<UnixTimestamp, Vec<QualifiedRoot>>,
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

    fn lowest_priority(&self) -> UnixTimestamp {
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
