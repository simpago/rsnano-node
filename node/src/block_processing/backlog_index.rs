use rsnano_core::{
    utils::{ContainerInfo, UnixTimestamp},
    Account, BlockHash, SavedBlock,
};
use std::collections::{BTreeMap, HashMap};

struct Entry {
    hash: BlockHash,
    account: Account,
    bucket_index: usize,
    priority: UnixTimestamp,
}

pub(super) struct BacklogIndex {
    by_hash: BTreeMap<BlockHash, Entry>,
    by_account: HashMap<Account, Vec<BlockHash>>,
    /// indexed by bucket index!
    /// Ordered in ascending timestamp order which means descending priority!
    by_priority: Vec<BTreeMap<UnixTimestamp, Vec<BlockHash>>>,
    bucket_lens: Vec<usize>,
}

impl BacklogIndex {
    pub fn insert(
        &mut self,
        block: &SavedBlock,
        bucket_index: usize,
        priority: UnixTimestamp,
    ) -> bool {
        let hash = block.hash();
        let account = block.account();

        let new_entry = Entry {
            hash,
            account,
            bucket_index,
            priority,
        };

        let mut inserted = true;
        self.by_hash
            .entry(hash)
            .and_modify(|_| inserted = false)
            .or_insert(new_entry);

        if inserted {
            self.by_account.entry(account).or_default().push(hash);
            self.by_priority[bucket_index]
                .entry(priority)
                .or_default()
                .push(hash);
            self.bucket_lens[bucket_index] += 1;
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
