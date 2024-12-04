#[cfg(test)]
use mock_instant::thread_local::Instant;
use rsnano_core::{BlockHash, HashOrAccount};
#[cfg(not(test))]
use std::time::Instant;
use std::{
    collections::{BTreeMap, HashMap},
    mem::size_of,
};

type AccountHead = [u8; 64];

pub struct PullsCache {
    max_cache_size: usize,
    by_account_head: HashMap<AccountHead, CachedPulls>,
    by_time: BTreeMap<Instant, Vec<AccountHead>>,
}

impl PullsCache {
    pub fn new() -> Self {
        Self::with_max_cache_size(10_000)
    }

    pub fn with_max_cache_size(max_cache_size: usize) -> Self {
        Self {
            by_account_head: HashMap::new(),
            by_time: BTreeMap::new(),
            max_cache_size,
        }
    }

    pub fn size(&self) -> usize {
        self.by_account_head.len()
    }

    pub const ELEMENT_SIZE: usize =
        size_of::<CachedPulls>() + size_of::<AccountHead>() * 2 + size_of::<Instant>();

    pub fn contains(&self, pull: &PullInfo) -> bool {
        self.by_account_head.contains_key(&to_head_512(pull))
    }

    pub fn add(&mut self, pull: &PullInfo) {
        if pull.processed <= 500 {
            return;
        }
        self.clean_old_pull();
        let head_512 = to_head_512(pull);
        if let Some(existing) = self.by_account_head.get_mut(&head_512) {
            update_cached_pull(head_512, existing, pull, &mut self.by_time);
        } else {
            self.insert_cached_pull(head_512, pull);
        }
    }

    fn insert_cached_pull(&mut self, head_512: AccountHead, pull: &PullInfo) {
        let time = Instant::now();
        let inserted = CachedPulls {
            time,
            new_head: pull.head,
        };
        self.by_account_head.insert(head_512, inserted);
        self.by_time.entry(time).or_default().push(head_512);
    }

    fn clean_old_pull(&mut self) {
        while self.size() >= self.max_cache_size {
            let (&time, heads) = self.by_time.iter_mut().next().unwrap();
            let head = heads.pop().unwrap();
            if heads.is_empty() {
                self.by_time.remove(&time);
            }
            self.by_account_head.remove(&head);
        }
    }

    pub fn update_pull(&self, pull: &mut PullInfo) {
        if let Some(existing) = self.by_account_head.get(&to_head_512(pull)) {
            pull.head = existing.new_head;
        }
    }

    pub fn remove(&mut self, pull: &PullInfo) {
        let head_512 = to_head_512(pull);
        if let Some(existing) = self.by_account_head.remove(&head_512) {
            let heads = self.by_time.get_mut(&existing.time).unwrap();
            heads.retain(|x| x != &head_512);
            if heads.is_empty() {
                self.by_time.remove(&existing.time);
            }
        }
    }
}

fn update_cached_pull(
    head_512: AccountHead,
    existing: &mut CachedPulls,
    pull_a: &PullInfo,
    by_time: &mut BTreeMap<Instant, Vec<[u8; 64]>>,
) {
    let old_time = existing.time;
    existing.time = Instant::now();
    existing.new_head = pull_a.head;
    let heads = by_time.get_mut(&old_time).unwrap();
    heads.retain(|x| x != &head_512);
    if heads.is_empty() {
        by_time.remove(&old_time);
    }
}

fn to_head_512(pull: &PullInfo) -> [u8; 64] {
    let mut head_512 = [0; 64];
    head_512[..32].copy_from_slice(pull.account_or_head.as_bytes());
    head_512[32..].copy_from_slice(pull.head_original.as_bytes());
    head_512
}

struct CachedPulls {
    time: Instant,
    new_head: BlockHash,
}

#[derive(Default, Clone)]
pub struct PullInfo {
    pub account_or_head: HashOrAccount,
    pub head: BlockHash,
    pub head_original: BlockHash,
    pub end: BlockHash,
    pub count: u32,
    pub attempts: u32,
    pub processed: u64,
    pub retry_limit: u32,
    pub bootstrap_id: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mock_instant::thread_local::MockClock;
    use std::time::Duration;

    #[test]
    fn empty() {
        let cache = PullsCache::new();
        assert_eq!(cache.size(), 0);
    }

    #[test]
    fn only_processed_above_500_is_cached() {
        let mut cache = PullsCache::new();

        cache.add(&PullInfo {
            processed: 499,
            ..test_pull(0)
        });
        assert_eq!(cache.size(), 0);

        cache.add(&PullInfo {
            processed: 500,
            ..test_pull(0)
        });
        assert_eq!(cache.size(), 0);

        let pull = PullInfo {
            processed: 501,
            ..test_pull(0)
        };
        cache.add(&pull);
        assert_eq!(cache.size(), 1);
        assert!(cache.contains(&pull))
    }

    #[test]
    fn remove_unknown_pull() {
        let mut cache = PullsCache::new();
        cache.remove(&test_pull(0));
        assert_eq!(cache.size(), 0);
    }

    #[test]
    fn remove() {
        let mut cache = PullsCache::new();
        cache.add(&test_pull(0));
        cache.add(&test_pull(1));
        cache.add(&test_pull(2));
        cache.remove(&test_pull(1));
        assert_eq!(cache.size(), 2);
        assert_eq!(cache.contains(&test_pull(1)), false);
    }

    #[test]
    fn cache_size_is_limited() {
        let mut cache = PullsCache::with_max_cache_size(2);
        cache.add(&test_pull(0));
        MockClock::advance(Duration::from_millis(1));
        cache.add(&test_pull(1));
        MockClock::advance(Duration::from_millis(1));
        cache.add(&test_pull(2));
        assert_eq!(cache.size(), 2);
        assert_eq!(cache.contains(&test_pull(0)), false);
    }

    #[test]
    fn update_pull_info_from_cache() {
        let mut cache = PullsCache::with_max_cache_size(2);
        let head = BlockHash::from_bytes([5; 32]);
        cache.add(&PullInfo {
            head,
            ..test_pull(0)
        });
        let mut pull = test_pull(0);
        cache.update_pull(&mut pull);
        assert_eq!(pull.head, head);
    }

    #[test]
    fn update_already_cached_pull() {
        let mut cache = PullsCache::with_max_cache_size(2);
        cache.add(&test_pull(0));
        let new_head = BlockHash::from_bytes([5; 32]);
        cache.add(&PullInfo {
            head: new_head,
            ..test_pull(0)
        });
        assert_eq!(cache.size(), 1);
        let mut pull = test_pull(0);
        cache.update_pull(&mut pull);
        assert_eq!(pull.head, new_head);
    }

    fn test_pull(id: u8) -> PullInfo {
        PullInfo {
            account_or_head: HashOrAccount::from_bytes([id; 32]),
            head: BlockHash::zero(),
            head_original: BlockHash::zero(),
            end: BlockHash::zero(),
            count: 0,
            attempts: 0,
            processed: 1000,
            retry_limit: 0,
            bootstrap_id: 0,
        }
    }
}
