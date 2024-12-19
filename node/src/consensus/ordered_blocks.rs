use rsnano_core::{BlockHash, SavedBlock};
use std::{
    cmp::Ordering,
    collections::{BTreeSet, HashSet},
};

pub(super) struct BlockEntry {
    pub time: u64,
    pub block: SavedBlock,
}

impl BlockEntry {
    pub fn hash(&self) -> BlockHash {
        self.block.hash()
    }
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

#[derive(Default)]
pub(super) struct OrderedBlocks {
    hashes: HashSet<BlockHash>,
    by_priority: BTreeSet<BlockEntry>,
}

impl OrderedBlocks {
    pub fn len(&self) -> usize {
        self.hashes.len()
    }

    pub fn contains(&self, hash: &BlockHash) -> bool {
        self.hashes.contains(hash)
    }

    pub fn insert(&mut self, entry: BlockEntry) -> bool {
        if self.hashes.contains(&entry.hash()) {
            return false;
        }

        self.hashes.insert(entry.hash());
        self.by_priority.insert(entry);
        true
    }

    pub fn first(&self) -> Option<&BlockEntry> {
        self.by_priority.first()
    }

    pub fn pop_first(&mut self) -> Option<BlockEntry> {
        let first = self.by_priority.pop_first()?;
        self.hashes.remove(&first.hash());
        Some(first)
    }

    pub fn pop_last(&mut self) -> Option<BlockEntry> {
        let last = self.by_priority.pop_last()?;
        self.hashes.remove(&last.hash());
        Some(last)
    }

    pub fn iter(&self) -> impl Iterator<Item = &BlockEntry> {
        self.by_priority.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty() {
        let blocks = OrderedBlocks::default();
        assert_eq!(blocks.len(), 0);
    }
}
