use rsnano_core::{
    utils::{ContainerInfo, UnixTimestamp},
    Account, BlockHash,
};
use std::collections::{BTreeMap, HashMap};

#[derive(Clone)]
pub(super) struct BacklogEntry {
    pub hash: BlockHash,
    pub account: Account,
    pub bucket_index: usize,
    pub priority: UnixTimestamp,
}

impl BacklogEntry {
    #[allow(dead_code)]
    pub fn new_test_instance() -> Self {
        Self {
            hash: 100.into(),
            account: 200.into(),
            bucket_index: 1,
            priority: UnixTimestamp::new(300),
        }
    }
}

pub(super) struct BacklogIndex {
    by_hash: BTreeMap<BlockHash, BacklogEntry>,
    by_account: HashMap<Account, Vec<BlockHash>>,
    /// indexed by bucket index!
    /// Ordered in ascending timestamp order which means descending priority!
    by_priority: Vec<BTreeMap<UnixTimestamp, Vec<BlockHash>>>,
    bucket_lens: Vec<usize>,
}

impl BacklogIndex {
    pub fn new(bucket_count: usize) -> Self {
        Self {
            by_hash: Default::default(),
            by_account: Default::default(),
            by_priority: vec![Default::default(); bucket_count],
            bucket_lens: vec![0; bucket_count],
        }
    }

    pub fn insert(&mut self, entry: BacklogEntry) -> bool {
        let mut inserted = true;
        self.by_hash
            .entry(entry.hash)
            .and_modify(|_| inserted = false)
            .or_insert(entry.clone());

        if inserted {
            self.by_account
                .entry(entry.account)
                .or_default()
                .push(entry.hash);
            self.by_priority[entry.bucket_index]
                .entry(entry.priority)
                .or_default()
                .push(entry.hash);
            self.bucket_lens[entry.bucket_index] += 1;
        }

        inserted
    }

    pub fn erase_account(&mut self, account: &Account) -> bool {
        let Some(hashes) = self.by_account.remove(account) else {
            return false;
        };

        for hash in hashes {
            let entry = self.by_hash.remove(&hash).unwrap();
            let prio_hashes = self.by_priority[entry.bucket_index]
                .get_mut(&entry.priority)
                .unwrap();
            if prio_hashes.len() == 1 {
                self.by_priority[entry.bucket_index].remove(&entry.priority);
            } else {
                prio_hashes.retain(|h| *h != hash);
            }
            self.bucket_lens[entry.bucket_index] -= 1;
        }
        true
    }

    pub fn erase_hash(&mut self, hash: &BlockHash) -> bool {
        let Some(entry) = self.by_hash.remove(hash) else {
            return false;
        };
        let account_hashes = self.by_account.get_mut(&entry.account).unwrap();
        if account_hashes.len() == 1 {
            self.by_account.remove(&entry.account);
        } else {
            account_hashes.retain(|h| *h != entry.hash);
        }
        let prio_hashes = self.by_priority[entry.bucket_index]
            .get_mut(&entry.priority)
            .unwrap();
        if prio_hashes.len() == 1 {
            self.by_priority[entry.bucket_index].remove(&entry.priority);
        } else {
            prio_hashes.retain(|h| *h != entry.hash);
        }
        self.bucket_lens[entry.bucket_index] -= 1;
        true
    }

    pub fn top(
        &self,
        bucket_index: usize,
        count: usize,
        mut filter: impl FnMut(&BlockHash) -> bool,
    ) -> Vec<BlockHash> {
        self.by_priority[bucket_index]
            .iter()
            .rev()
            .flat_map(|(_, hashes)| hashes)
            .cloned()
            .filter(|hash| filter(hash))
            .take(count)
            .collect()
    }

    pub fn next(&self, last: &BlockHash, count: usize) -> Vec<BlockHash> {
        let Some(start) = last.inc() else {
            return Vec::new();
        };
        self.by_hash
            .range(start..)
            .map(|(hash, _)| *hash)
            .take(count)
            .collect()
    }

    pub fn contains(&self, hash: &BlockHash) -> bool {
        self.by_hash.contains_key(hash)
    }

    pub fn len(&self) -> usize {
        self.by_hash.len()
    }

    pub fn len_of_bucket(&self, bucket_index: usize) -> usize {
        self.bucket_lens[bucket_index]
    }

    pub fn container_info(&self) -> ContainerInfo {
        let mut bucket_sizes = ContainerInfo::builder();
        for (index, len) in self.bucket_lens.iter().enumerate() {
            bucket_sizes = bucket_sizes.leaf(index.to_string(), *len, 0);
        }

        ContainerInfo::builder()
            .leaf("blocks", self.len(), 0)
            .node("sizes", bucket_sizes.finish())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const TEST_BUCKET_COUNT: usize = 5;

    #[test]
    fn empty() {
        let mut index = BacklogIndex::new(TEST_BUCKET_COUNT);
        assert_eq!(index.len(), 0);
        assert_eq!(index.contains(&BlockHash::from(1)), false);

        assert!(index.top(0, 100, |_| unreachable!()).is_empty());
        assert!(index.next(&BlockHash::zero(), 100).is_empty());

        assert_eq!(index.erase_hash(&BlockHash::from(1)), false);
        assert_eq!(index.erase_account(&Account::from(1)), false);
    }

    #[test]
    fn insert_one() {
        let mut index = BacklogIndex::new(TEST_BUCKET_COUNT);
        let entry = BacklogEntry::new_test_instance();

        let inserted = index.insert(entry.clone());

        assert!(inserted);
        assert_eq!(index.len(), 1);
        assert!(index.contains(&entry.hash));
        assert_eq!(index.len_of_bucket(entry.bucket_index), 1);
        assert_eq!(index.len_of_bucket(entry.bucket_index + 1), 0);
    }

    #[test]
    fn insert_two() {
        let mut index = BacklogIndex::new(TEST_BUCKET_COUNT);
        let entry1 = BacklogEntry::new_test_instance();
        let entry2 = BacklogEntry {
            account: 1000.into(),
            hash: 1001.into(),
            ..BacklogEntry::new_test_instance()
        };

        index.insert(entry1.clone());
        let inserted = index.insert(entry2.clone());

        assert!(inserted);
        assert_eq!(index.len(), 2);
        assert!(index.contains(&entry1.hash));
        assert!(index.contains(&entry2.hash));
        assert_eq!(index.len_of_bucket(entry1.bucket_index), 2);
    }

    #[test]
    fn insert_many() {
        let mut index = BacklogIndex::new(TEST_BUCKET_COUNT);

        let entry1 = BacklogEntry {
            account: 1000.into(),
            hash: 1001.into(),
            ..BacklogEntry::new_test_instance()
        };

        let entry2 = BacklogEntry {
            account: 2000.into(),
            hash: 2001.into(),
            priority: UnixTimestamp::new(1),
            ..BacklogEntry::new_test_instance()
        };

        let entry3 = BacklogEntry {
            account: 2000.into(),
            hash: 2002.into(),
            priority: UnixTimestamp::new(1),
            ..BacklogEntry::new_test_instance()
        };

        let entry4 = BacklogEntry {
            account: 2000.into(),
            hash: 2003.into(),
            priority: UnixTimestamp::new(2),
            ..BacklogEntry::new_test_instance()
        };

        assert!(index.insert(entry1));
        assert!(index.insert(entry2));
        assert!(index.insert(entry3));
        assert!(index.insert(entry4));

        assert_eq!(index.len(), 4);
        assert_eq!(index.by_priority.iter().map(|i| i.len()).sum::<usize>(), 3);
        assert_eq!(index.by_account.values().map(|i| i.len()).sum::<usize>(), 4);
    }

    #[test]
    fn dont_insert_duplicate() {
        let mut index = BacklogIndex::new(TEST_BUCKET_COUNT);
        let entry = BacklogEntry::new_test_instance();

        index.insert(entry.clone());
        let inserted = index.insert(entry.clone());

        assert!(!inserted);
        assert_eq!(index.len(), 1);
    }

    #[test]
    fn erase_account() {
        let mut index = BacklogIndex::new(TEST_BUCKET_COUNT);

        let entry1 = BacklogEntry {
            account: 1000.into(),
            hash: 1001.into(),
            ..BacklogEntry::new_test_instance()
        };

        let entry2 = BacklogEntry {
            account: 2000.into(),
            hash: 2001.into(),
            priority: UnixTimestamp::new(1),
            ..BacklogEntry::new_test_instance()
        };

        let entry3 = BacklogEntry {
            account: 2000.into(),
            hash: 2002.into(),
            priority: UnixTimestamp::new(1),
            ..BacklogEntry::new_test_instance()
        };

        let entry4 = BacklogEntry {
            account: 2000.into(),
            hash: 2003.into(),
            priority: UnixTimestamp::new(2),
            ..BacklogEntry::new_test_instance()
        };

        index.insert(entry1.clone());
        index.insert(entry2.clone());
        index.insert(entry3);
        index.insert(entry4);

        index.erase_account(&entry2.account);

        assert_eq!(index.len(), 1);
        assert!(index.contains(&entry1.hash));
        assert_eq!(index.by_priority.iter().map(|i| i.len()).sum::<usize>(), 1);
        assert_eq!(index.by_account.values().map(|i| i.len()).sum::<usize>(), 1);
    }

    #[test]
    fn erase_by_hash() {
        let mut index = BacklogIndex::new(TEST_BUCKET_COUNT);

        let entry1 = BacklogEntry {
            account: 1000.into(),
            hash: 1001.into(),
            ..BacklogEntry::new_test_instance()
        };

        let entry2 = BacklogEntry {
            account: 2000.into(),
            hash: 2001.into(),
            priority: UnixTimestamp::new(1),
            ..BacklogEntry::new_test_instance()
        };

        let entry3 = BacklogEntry {
            account: 2000.into(),
            hash: 2002.into(),
            priority: UnixTimestamp::new(1),
            ..BacklogEntry::new_test_instance()
        };

        index.insert(entry1.clone());
        index.insert(entry2.clone());
        index.insert(entry3.clone());

        index.erase_hash(&entry1.hash);
        index.erase_hash(&entry2.hash);

        assert_eq!(index.len(), 1);
        assert!(index.contains(&entry3.hash));
        assert_eq!(index.by_priority.iter().map(|i| i.len()).sum::<usize>(), 1);
        assert_eq!(index.by_account.values().map(|i| i.len()).sum::<usize>(), 1);
    }

    #[test]
    fn top() {
        let mut index = BacklogIndex::new(TEST_BUCKET_COUNT);

        let entry1 = BacklogEntry {
            account: 1000.into(),
            hash: 1001.into(),
            bucket_index: 3,
            priority: UnixTimestamp::new(1),
        };

        let entry2 = BacklogEntry {
            account: 2000.into(),
            hash: 2001.into(),
            bucket_index: 3,
            priority: UnixTimestamp::new(10000),
        };

        // Same priority as entry2
        let entry3 = BacklogEntry {
            account: 3000.into(),
            hash: 3001.into(),
            bucket_index: 3,
            priority: UnixTimestamp::new(2),
        };

        // filtered out!
        let entry4 = BacklogEntry {
            account: 4000.into(),
            hash: 4001.into(),
            bucket_index: 3,
            priority: UnixTimestamp::new(2),
        };

        // different bucket
        let entry5 = BacklogEntry {
            account: 5000.into(),
            hash: 5001.into(),
            priority: UnixTimestamp::new(2),
            bucket_index: 1,
        };

        index.insert(entry1.clone());
        index.insert(entry2.clone());
        index.insert(entry3.clone());
        index.insert(entry4.clone());
        index.insert(entry5.clone());

        let top_all = index.top(3, usize::MAX, |h| *h != entry4.hash);

        // ordered by ascending priority (=descending timestamp)
        assert_eq!(top_all, vec![entry2.hash, entry3.hash, entry1.hash]);

        let top_limit = index.top(3, 2, |h| *h != entry4.hash);
        assert_eq!(top_limit, vec![entry2.hash, entry3.hash]);
    }

    #[test]
    fn next() {
        let mut index = BacklogIndex::new(TEST_BUCKET_COUNT);

        let entry1 = BacklogEntry {
            account: 1000.into(),
            hash: 1001.into(),
            bucket_index: 3,
            priority: UnixTimestamp::new(1),
        };

        let entry2 = BacklogEntry {
            account: 2000.into(),
            hash: 2001.into(),
            bucket_index: 3,
            priority: UnixTimestamp::new(10000),
        };

        // Same priority as entry2
        let entry3 = BacklogEntry {
            account: 3000.into(),
            hash: 3001.into(),
            bucket_index: 3,
            priority: UnixTimestamp::new(2),
        };

        index.insert(entry1.clone());
        index.insert(entry2.clone());
        index.insert(entry3.clone());

        let next_all = index.next(&entry1.hash, usize::MAX);
        assert_eq!(next_all, vec![entry2.hash, entry3.hash]);

        let next_limit = index.next(&entry1.hash, 1);
        assert_eq!(next_limit, vec![entry2.hash]);
    }

    #[test]
    fn next_max_hash() {
        let mut index = BacklogIndex::new(TEST_BUCKET_COUNT);
        index.insert(BacklogEntry::new_test_instance());
        let next = index.next(&BlockHash::MAX, usize::MAX);
        assert!(next.is_empty());
    }

    #[test]
    fn container_info() {
        let mut index = BacklogIndex::new(TEST_BUCKET_COUNT);

        let entry1 = BacklogEntry {
            account: 1000.into(),
            hash: 1001.into(),
            ..BacklogEntry::new_test_instance()
        };

        let entry2 = BacklogEntry {
            account: 2000.into(),
            hash: 2001.into(),
            priority: UnixTimestamp::new(1),
            ..BacklogEntry::new_test_instance()
        };

        let entry3 = BacklogEntry {
            account: 2000.into(),
            hash: 2002.into(),
            priority: UnixTimestamp::new(1),
            ..BacklogEntry::new_test_instance()
        };

        index.insert(entry1.clone());
        index.insert(entry2.clone());
        index.insert(entry3.clone());

        let container_info = index.container_info();
        let expected_sizes: ContainerInfo = [
            ("0", 0, 0),
            ("1", 3, 0),
            ("2", 0, 0),
            ("3", 0, 0),
            ("4", 0, 0),
        ]
        .into();
        let expected = ContainerInfo::builder()
            .leaf("blocks", 3, 0)
            .node("sizes", expected_sizes)
            .finish();
        assert_eq!(container_info, expected);
    }
}
