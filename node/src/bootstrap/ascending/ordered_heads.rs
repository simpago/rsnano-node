use rsnano_core::Account;
use rsnano_nullable_clock::Timestamp;
use std::collections::{BTreeMap, BTreeSet};

/// Represents a range of accounts to scan, once the full range is scanned (goes past `end`)
/// the head wraps around (to the `start`)
pub(super) struct FrontierHead {
    /// The range of accounts to scan is [start, end)
    pub start: Account,
    pub end: Account,

    /// We scan the range by querying frontiers starting at 'next' and gathering candidates
    pub next: Account,
    pub candidates: BTreeSet<Account>,

    pub requests: usize,
    pub completed: usize,
    pub timestamp: Timestamp,

    /// Total number of accounts processed
    pub processed: usize,
}

impl FrontierHead {
    pub fn new(start: Account, end: Account) -> Self {
        Self {
            start,
            end,
            next: start,
            candidates: Default::default(),
            requests: 0,
            completed: 0,
            timestamp: Timestamp::default(),
            processed: 0,
        }
    }
}

#[derive(Default)]
pub(super) struct OrderedHeads {
    sequenced: Vec<Account>,
    by_start: BTreeMap<Account, FrontierHead>,
    by_timestamp: BTreeMap<Timestamp, Vec<Account>>,
}

impl OrderedHeads {
    pub fn push_back(&mut self, head: FrontierHead) {
        let start = head.start;
        let timestamp = head.timestamp;
        let mut inserted = true;
        self.by_start
            .entry(start)
            .and_modify(|_| inserted = false)
            .or_insert(head);

        if !inserted {
            return;
        }
        self.sequenced.push(start);
        self.by_timestamp.entry(timestamp).or_default().push(start);
    }

    pub fn ordered_by_timestamp(&self) -> impl Iterator<Item = &FrontierHead> {
        self.by_timestamp
            .values()
            .flatten()
            .map(|start| self.by_start.get(start).unwrap())
    }

    pub fn modify<F>(&mut self, start: &Account, mut f: F)
    where
        F: FnMut(&mut FrontierHead),
    {
        if let Some(head) = self.by_start.get_mut(start) {
            let old_timestamp = head.timestamp;
            f(head);
            if head.timestamp != old_timestamp {
                let accounts = self.by_timestamp.get_mut(&old_timestamp).unwrap();
                if accounts.len() == 1 {
                    self.by_timestamp.remove(&old_timestamp);
                } else {
                    accounts.retain(|a| a != start);
                }
                self.by_timestamp
                    .entry(head.timestamp)
                    .or_default()
                    .push(*start);
            }
        } else {
            panic!("head not found: {}", start.encode_account());
        }
    }

    pub fn find_first_less_than_or_equal_to(&self, account: Account) -> Option<Account> {
        self.by_start
            .range(..=account)
            .last()
            .map(|(start, _)| *start)
    }

    pub fn iter(&self) -> impl Iterator<Item = &FrontierHead> {
        self.by_start.values()
    }

    pub fn len(&self) -> usize {
        self.sequenced.len()
    }
}
