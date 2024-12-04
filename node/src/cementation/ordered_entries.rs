use crate::consensus::Election;
use rsnano_core::BlockHash;
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

#[derive(Default)]
pub(super) struct OrderedEntries {
    sequenced: VecDeque<BlockHash>,
    by_hash: HashMap<BlockHash, Entry>,
}

impl OrderedEntries {
    pub fn insert(&mut self, entry: Entry) -> bool {
        let hash = entry.hash;
        let mut inserted = true;

        self.by_hash
            .entry(hash)
            .and_modify(|_| {
                inserted = false;
            })
            .or_insert(entry);

        if inserted {
            self.sequenced.push_back(hash);
        }

        inserted
    }

    pub(crate) fn contains(&self, hash: &BlockHash) -> bool {
        self.by_hash.contains_key(hash)
    }

    pub(crate) fn len(&self) -> usize {
        self.sequenced.len()
    }

    pub(crate) fn pop_front(&mut self) -> Option<Entry> {
        if let Some(hash) = self.sequenced.pop_front() {
            self.by_hash.remove(&hash)
        } else {
            None
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.sequenced.is_empty()
    }
}

pub(super) struct Entry {
    pub hash: BlockHash,
    pub election: Option<Arc<Election>>,
}
