use std::{
    collections::{BTreeMap, HashMap},
    mem::size_of,
    sync::{Mutex, OnceLock},
};

use rsnano_core::{
    utils::{ContainerInfo, ContainerInfoComponent},
    Account, BlockHash,
};

use crate::voting::Vote;

pub struct Config {
    max_size: usize,
}

impl Config {
    pub fn new(max_size: usize) -> Self {
        Config { max_size }
    }

    pub fn create_null() -> Self {
        Config::new(1024)
    }
}

#[derive(Default, Debug, Clone)]
pub struct CacheEntry {
    id: usize,
    hash: BlockHash,
    voters: Vec<(Account, u64)>,
    tally: u128,
}

impl CacheEntry {
    const MAX_VOTERS: usize = 40;

    pub fn new(hash: BlockHash) -> Self {
        CacheEntry {
            id: 0,
            hash,
            voters: vec![],
            tally: 0,
        }
    }

    pub fn vote(&mut self, representative: &Account, timestamp: u64, rep_weight: u128) -> bool {
        if let Some(existing) = self
            .voters
            .iter_mut()
            .find(|(key, _)| key == representative)
        {
            // We already have a vote from this rep
            // Update timestamp if newer but tally remains unchanged as we already counted this rep weight
            // It is not essential to keep tally up to date if rep voting weight changes, elections do tally calculations independently, so in the worst case scenario only our queue ordering will be a bit off
            if timestamp > existing.1 {
                existing.1 = timestamp
            }
            return false;
        }
        // Vote from an unseen representative, add to list and update tally
        if self.voters.len() < Self::MAX_VOTERS {
            self.voters.push((*representative, timestamp));
            self.tally += rep_weight;
            return true;
        }
        false
    }

    // FIXME: depends on nano::election -> port first
    // pub fn fill()

    pub fn size(&self) -> usize {
        self.voters.len()
    }
}

#[derive(Debug, Clone)]
pub struct QueueEntry {
    id: usize,
    hash: BlockHash,
    tally: u128,
}

impl QueueEntry {
    pub fn new(hash: BlockHash, tally: u128) -> Self {
        QueueEntry { id: 0, hash, tally }
    }
}

#[derive(Debug, Clone)]
struct CacheEntryContainer {
    next_id: usize,
    by_id: BTreeMap<usize, CacheEntry>,
    by_hash: BTreeMap<BlockHash, usize>,
}

impl CacheEntryContainer {
    fn new() -> Self {
        Self {
            next_id: 0,
            by_id: BTreeMap::new(),
            by_hash: BTreeMap::new(),
        }
    }

    fn pop_front(&mut self) -> Option<CacheEntry> {
        let (_id, entry) = self.by_id.pop_first()?;
        self.remove(&entry);
        Some(entry)
    }

    fn len(&self) -> usize {
        self.by_id.len()
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn size_of_element() -> usize {
        size_of::<Self>()
    }

    fn create_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        id
    }

    pub fn insert(&mut self, mut entry: CacheEntry) -> bool {
        entry.id = self.create_id();

        self.by_id.insert(entry.id, entry.clone());
        self.by_hash.insert(entry.hash, entry.id);

        true
    }

    fn remove(&mut self, entry: &CacheEntry) -> bool {
        let result = self.by_id.remove(&entry.id).is_some();
        self.by_hash.remove(&entry.hash);

        result
    }
}

#[derive(Default, Clone, Debug)]
struct QueueEntryContainer {
    next_id: usize,
    by_id: BTreeMap<usize, QueueEntry>,
    by_tally: BTreeMap<u128, Vec<usize>>,
    by_hash: BTreeMap<BlockHash, usize>,
}

impl QueueEntryContainer {
    fn new() -> Self {
        Self {
            next_id: 0,
            by_id: BTreeMap::new(),
            by_tally: BTreeMap::new(),
            by_hash: BTreeMap::new(),
        }
    }

    fn by_hash(&self, hash: &BlockHash) -> Option<&QueueEntry> {
        let id = self.by_hash.get(hash)?;
        self.by_id.get(id)
    }

    fn by_tally_last_entry(&self) -> Option<&QueueEntry> {
        let (_, key) = self.by_tally.last_key_value()?;
        self.by_id.get(key.last()?)
    }

    fn by_tally_erase(&mut self, entry: &QueueEntry) {
        self.by_id.remove(&entry.id);
        self.by_hash.remove(&entry.hash);

        if let Some(by_tally) = self.by_tally.get_mut(&entry.tally) {
            // if by_tally only has one element left, remove key
            match by_tally.len() {
                1 => {
                    self.by_tally.remove(&entry.tally);
                }
                _ => {
                    by_tally.retain(|id| *id != entry.id);
                }
            }
        }
    }

    fn pop_front(&mut self) -> Option<QueueEntry> {
        let (_id, entry) = self.by_id.pop_first()?;
        self.remove(&entry);
        Some(entry)
    }

    fn len(&self) -> usize {
        self.by_id.len()
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn size_of_element() -> usize {
        size_of::<Self>()
    }

    fn create_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        id
    }

    pub fn insert(&mut self, mut entry: QueueEntry) -> bool {
        entry.id = self.create_id();

        self.by_id.insert(entry.id, entry.clone());
        self.by_hash.insert(entry.hash, entry.id);
        match self.by_tally.get_mut(&entry.tally) {
            None => {
                self.by_tally.insert(entry.tally, vec![entry.id]);
            }
            Some(by_tally_entry) => {
                by_tally_entry.push(entry.id);
            }
        }
        self.next_id = self.next_id.wrapping_add(1);

        true
    }

    pub fn remove(&mut self, entry: &QueueEntry) -> bool {
        let result = self.by_id.remove(&entry.id).is_some();

        self.by_hash.remove(&entry.hash);
        if let Some(ids) = self.by_tally.get_mut(&entry.tally) {
            if ids.len() == 1 {
                self.by_tally.remove(&entry.tally);
            } else {
                ids.retain(|id| *id != entry.id);
            }
        }

        result
    }
}

#[derive(Debug)]

pub struct VoteCache {
    max_size: usize,
    cache: CacheEntryContainer,
    queue: QueueEntryContainer,
}

impl VoteCache {
    pub fn new(config: Config) -> Self {
        VoteCache {
            max_size: config.max_size,
            cache: CacheEntryContainer::new(),
            queue: QueueEntryContainer::new(),
        }
    }

    pub fn create_null() -> Self {
        let config = Config::create_null();
        VoteCache::new(config)
    }

    fn rep_to_weight_map() -> &'static Mutex<HashMap<Account, u128>> {
        static REP_TO_WEIGHT_MAP: OnceLock<Mutex<HashMap<Account, u128>>> = OnceLock::new();
        REP_TO_WEIGHT_MAP.get_or_init(|| Mutex::new(HashMap::new()))
    }

    pub fn vote(&mut self, hash: &BlockHash, vote: &Vote) {
        let weight = self.rep_weight_query()(&vote.voting_account);
        self.vote_impl(hash, &vote.voting_account, vote.timestamp(), weight);
    }

    pub fn find(&self, hash: &BlockHash) -> Option<&CacheEntry> {
        self.find_locked(hash)
    }

    pub fn remove(&mut self, hash: &BlockHash) -> bool {
        let mut result = false;
        if let Some(existing) = self
            .cache
            .by_hash
            .get(hash)
            .map(|id| self.cache.by_id.get(id).unwrap().clone())
        {
            self.cache.remove(&existing);
            result = true;
        }

        if let Some(existing) = self
            .queue
            .by_hash
            .get(hash)
            .map(|id| self.queue.by_id.get(id).unwrap().clone())
        {
            self.queue.remove(&existing);
            result = true;
        }

        result
    }

    pub fn peek(&self, min_tally: Option<u128>) -> Option<&CacheEntry> {
        if self.queue.is_empty() {
            return None;
        }

        let (_, ids) = self.queue.by_tally.last_key_value()?; // element with the highest tally;
        let top = self.queue.by_id.get(ids.last().unwrap())?;
        let cache_entry = self.find_locked(&top.hash)?;

        match cache_entry.tally >= min_tally.unwrap_or(0) {
            true => Some(cache_entry),
            false => None,
        }
    }

    pub fn pop(&mut self, min_tally: Option<u128>) -> Option<CacheEntry> {
        if self.queue.is_empty() {
            return None;
        };

        let (_, ids) = self.queue.by_tally.last_key_value()?;
        let top = self.queue.by_id.get(ids.last().unwrap())?.clone();
        let cache_entry = self.find_locked(&top.hash)?.to_owned();
        if cache_entry.tally < min_tally.unwrap_or_default() {
            return None;
        }

        self.queue.by_tally_erase(&top);
        Some(cache_entry)
    }

    pub fn trigger(&mut self, hash: &BlockHash) {
        if self.queue.by_hash(hash).is_some() {
            if let Some(existing_cache_entry) = self.find_locked(hash) {
                self.queue
                    .insert(QueueEntry::new(*hash, existing_cache_entry.tally));
                self.trim_overflow_locked();
            }
        }
    }

    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }

    pub fn queue_size(&self) -> usize {
        self.queue.len()
    }

    pub fn cache_empty(&self) -> bool {
        self.cache.is_empty()
    }

    pub fn queue_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn collect_container_info(&self, name: String) -> ContainerInfoComponent {
        let children = vec![
            ContainerInfoComponent::Leaf(ContainerInfo {
                name: "cache".to_owned(),
                count: self.cache_size(),
                sizeof_element: CacheEntryContainer::size_of_element(),
            }),
            ContainerInfoComponent::Leaf(ContainerInfo {
                name: "queue".to_owned(),
                count: self.queue_size(),
                sizeof_element: QueueEntryContainer::size_of_element(),
            }),
        ];

        ContainerInfoComponent::Composite(name, children)
    }

    pub fn rep_weight_query(&self) -> impl Fn(&Account) -> u128 {
        |acc| match Self::rep_to_weight_map().lock().unwrap().get(acc) {
            Some(weight) => *weight,
            None => 0,
        }
    }

    fn vote_impl(
        &mut self,
        hash: &BlockHash,
        representative: &Account,
        timestamp: u64,
        rep_weight: u128,
    ) {
        /*
         * If there is no cache entry for the block hash, create a new entry for both cache and queue.
         * Otherwise update existing cache entry and, if queue contains entry for the block hash, update the queue entry
         */
        // auto & cache_by_hash = cache.get<tag_hash> ();
        if let Some(existing) = self
            .cache
            .by_hash
            .get(hash)
            .map(|id| self.cache.by_id.get_mut(id).unwrap())
        {
            existing.vote(representative, timestamp, rep_weight);

            if let Some(ent) = self
                .queue
                .by_hash
                .get(hash)
                .map(|id| self.queue.by_id.get_mut(id).unwrap())
            {
                let by_tally_ids = self.queue.by_tally.get_mut(&ent.tally).unwrap();
                // if only one id is left -> remove key entriely
                if by_tally_ids.len() == 1 {
                    self.queue.by_tally.remove(&ent.tally);
                } else {
                    // else filter out all other ids
                    by_tally_ids.retain(|id| *id != ent.id);
                }

                ent.tally = existing.tally;
                match self.queue.by_tally.get_mut(&ent.tally) {
                    Some(by_tally_ids) => {
                        by_tally_ids.insert(0, ent.id);
                    }
                    None => {
                        self.queue.by_tally.insert(ent.tally, vec![ent.id]);
                    }
                }
            }
        } else {
            let mut cache_entry = CacheEntry::new(*hash);
            cache_entry.vote(representative, timestamp, rep_weight);

            let queue_entry = QueueEntry::new(*hash, cache_entry.tally);
            self.cache.insert(cache_entry);

            if let Some(queue_existing) = self.queue.clone().by_hash(hash) {
                self.queue.remove(queue_existing);
            }
            self.queue.insert(queue_entry);

            self.trim_overflow_locked();
        }
    }

    fn find_locked(&self, hash: &BlockHash) -> Option<&CacheEntry> {
        let id = self.cache.by_hash.get(hash)?;
        self.cache.by_id.get(id)
    }

    fn trim_overflow_locked(&mut self) {
        // When cache overflown remove the oldest entry
        if self.cache.len() > self.max_size {
            self.cache.pop_front();
        }

        if self.queue.len() > self.max_size {
            self.queue.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use rsnano_core::KeyPair;

    use super::*;

    fn register_rep(rep: Account, weight: u128) {
        let mut map = VoteCache::rep_to_weight_map().lock().unwrap();
        map.insert(rep, weight);
    }

    fn create_rep(weight: u128) -> KeyPair {
        let key = KeyPair::new();
        register_rep(key.public_key(), weight);
        key
    }

    fn create_vote(rep: &KeyPair, hash: &BlockHash, timestamp_offset: u64) -> Vote {
        Vote::new(
            rep.public_key(),
            &rep.private_key(),
            timestamp_offset * 1024 * 1024,
            0,
            vec![*hash],
        )
    }

    fn vote(cache: &mut VoteCache, vote: &Vote) {
        cache.vote(vote.hashes.first().unwrap(), vote)
    }

    fn pop(cache: &mut VoteCache) -> CacheEntry {
        let pop = cache.pop(None);
        assert!(pop.is_some());
        pop.unwrap()
    }

    fn peek(cache: &VoteCache) -> &CacheEntry {
        let peek = cache.peek(None);
        assert!(peek.is_some());
        peek.unwrap()
    }

    #[test]
    fn construction() {
        let cache = VoteCache::create_null();
        assert_eq!(cache.cache_size(), 0);
        assert!(cache.cache_empty());
        let hash = BlockHash::random();
        assert!(cache.find(&hash).is_none());
    }

    #[test]
    fn insert_one_hash() {
        let mut cache = VoteCache::create_null();

        let rep1 = create_rep(7);
        let hash1 = BlockHash::random();
        let vote1 = create_vote(&rep1, &hash1, 1);

        vote(&mut cache, &vote1);
        assert_eq!(cache.cache_size(), 1);
        assert!(cache.find(&hash1).is_some());
        let peek1 = peek(&cache);
        assert_eq!(peek1.hash, hash1);
        assert_eq!(peek1.voters.len(), 1);
        assert_eq!(
            peek1.voters.first(),
            Some(&(rep1.public_key(), 1024 * 1024))
        );
        assert_eq!(peek1.tally, 7)
    }

    /*
     * Inserts multiple votes for single hash
     * Ensures all of them can be retrieved and that tally is properly accumulated
     */
    #[test]
    fn insert_one_hash_many_votes() {
        let mut cache = VoteCache::create_null();

        let hash1 = BlockHash::random();
        let rep1 = create_rep(7);
        let rep2 = create_rep(9);
        let rep3 = create_rep(11);

        let vote1 = create_vote(&rep1, &hash1, 1);
        let vote2 = create_vote(&rep2, &hash1, 2);
        let vote3 = create_vote(&rep3, &hash1, 3);

        vote(&mut cache, &vote1);
        vote(&mut cache, &vote2);
        vote(&mut cache, &vote3);
        // We have 3 votes but for a single hash, so just one entry in vote cache
        assert_eq!(cache.cache_size(), 1);
        let peek1 = peek(&cache);
        assert_eq!(peek1.voters.len(), 3);
        // Tally must be the sum of rep weights
        assert_eq!(peek1.tally, 7 + 9 + 11);
    }

    #[test]
    fn insert_many_hashes_many_votes() {
        let mut cache = VoteCache::create_null();

        // There will be 3 random hashes to vote for
        let hash1 = BlockHash::random();
        let hash2 = BlockHash::random();
        let hash3 = BlockHash::random();

        // There will be 4 reps with different weights
        let rep1 = create_rep(7);
        let rep2 = create_rep(9);
        let rep3 = create_rep(11);
        let rep4 = create_rep(13);

        // Votes: rep1 > hash1, rep2 > hash2, rep3 > hash3, rep4 > hash1 (the same as rep1)
        let vote1 = create_vote(&rep1, &hash1, 1);
        let vote2 = create_vote(&rep2, &hash2, 1);
        let vote3 = create_vote(&rep3, &hash3, 1);
        let vote4 = create_vote(&rep4, &hash1, 1);

        // Insert first 3 votes in cache
        vote(&mut cache, &vote1);
        vote(&mut cache, &vote2);
        vote(&mut cache, &vote3);

        // Ensure all of those are properly inserted
        assert_eq!(cache.cache_size(), 3);
        assert!(cache.find(&hash1).is_some());
        assert!(cache.find(&hash2).is_some());
        assert!(cache.find(&hash3).is_some());

        // Ensure that first entry in queue is the one for hash3 (rep3 has the highest weight of the first 3 reps)
        let peek1 = peek(&cache);
        assert_eq!(peek1.voters.len(), 1);
        assert_eq!(peek1.tally, 11);
        assert_eq!(peek1.hash, hash3);

        // Now add a vote from rep4 with the highest voting weight
        vote(&mut cache, &vote4);

        // Ensure that the first entry in queue is now the one for hash1 (rep1 + rep4 tally weight)
        let pop1 = pop(&mut cache);
        assert_eq!(pop1.voters.len(), 2);
        assert_eq!(pop1.tally, 7 + 13);
        assert_eq!(pop1.hash, hash1);
        assert!(cache.find(&hash1).is_some()); // Only pop from queue, votes should still be stored in cache

        // After popping the previous entry, the next entry in queue should be hash3 (rep3 tally weight)
        let pop2 = pop(&mut cache);
        assert_eq!(pop2.voters.len(), 1);
        assert_eq!(pop2.tally, 11);
        assert_eq!(pop2.hash, hash3);
        assert!(cache.find(&hash3).is_some());

        // And last one should be hash2 with rep2 tally weight
        let pop3 = pop(&mut cache);
        assert_eq!(pop3.voters.len(), 1);
        assert_eq!(pop3.tally, 9);
        assert_eq!(pop3.hash, hash2);
        assert!(cache.find(&hash2).is_some());

        assert!(cache.queue_empty());
    }

    /*
     * Ensure that duplicate votes are ignored
     */
    #[test]
    fn insert_duplicate() {
        let mut cache = VoteCache::create_null();

        let hash1 = BlockHash::random();
        let rep1 = create_rep(9);
        let vote1 = create_vote(&rep1, &hash1, 1);
        let vote2 = create_vote(&rep1, &hash1, 1);

        vote(&mut cache, &vote1);
        vote(&mut cache, &vote2);

        assert_eq!(cache.cache_size(), 1)
    }

    /*
     * Ensure that when processing vote from a representative that is already cached, we always update to the vote with the highest timestamp
     */
    #[test]
    fn insert_newer() {
        let mut cache = VoteCache::create_null();

        let hash1 = BlockHash::random();
        let rep1 = create_rep(9);
        let vote1 = create_vote(&rep1, &hash1, 1);
        vote(&mut cache, &vote1);
        let peek1 = peek(&cache).clone();

        const DURATION_MAX: u8 = 0x0F;
        const TIMESTAMP_MAX: u64 = 0xFFFF_FFFF_FFFF_FFF0;

        let vote2 = Vote::new(
            rep1.public_key(),
            &rep1.private_key(),
            TIMESTAMP_MAX,
            DURATION_MAX,
            vec![hash1],
        );
        vote(&mut cache, &vote2);

        let peek2 = peek(&cache);
        assert_eq!(cache.cache_size(), 1);
        assert_eq!(peek2.voters.len(), 1);
        // Second entry should have timestamp greater than the first one
        assert!(peek2.voters.first().unwrap().1 > peek1.voters.first().unwrap().1);
        assert_eq!(peek2.voters.first().unwrap().1, u64::MAX); // final timestamp
    }

    /*
     * Ensure that when processing vote from a representative that is already cached, votes with older timestamp are ignored
     */
    #[test]
    fn insert_older() {
        let mut cache = VoteCache::create_null();
        let hash1 = BlockHash::random();
        let rep1 = create_rep(9);
        let vote1 = create_vote(&rep1, &hash1, 2);
        vote(&mut cache, &vote1);
        let peek1 = peek(&cache).clone();

        let vote2 = create_vote(&rep1, &hash1, 1);
        vote(&mut cache, &vote2);
        let peek2 = peek(&cache);

        assert_eq!(cache.cache_size(), 1);
        assert_eq!(peek2.voters.len(), 1);
        assert_eq!(
            peek2.voters.first().unwrap().1,
            peek1.voters.first().unwrap().1
        ); // timestamp2 == timestamp1
    }

    /*
     * Ensure that erase functionality works
     */
    #[test]
    fn erase() {
        let mut cache = VoteCache::create_null();
        let hash1 = BlockHash::random();
        let hash2 = BlockHash::random();
        let hash3 = BlockHash::random();

        let rep1 = create_rep(7);
        let rep2 = create_rep(9);
        let rep3 = create_rep(11);

        let vote1 = create_vote(&rep1, &hash1, 1);
        let vote2 = create_vote(&rep2, &hash2, 1);
        let vote3 = create_vote(&rep3, &hash3, 1);

        vote(&mut cache, &vote1);
        vote(&mut cache, &vote2);
        vote(&mut cache, &vote3);

        assert_eq!(cache.cache_size(), 3);
        assert!(cache.find(&hash1).is_some());
        assert!(cache.find(&hash2).is_some());
        assert!(cache.find(&hash3).is_some());

        cache.remove(&hash2);

        assert_eq!(cache.cache_size(), 2);
        assert!(cache.find(&hash1).is_some());
        assert!(cache.find(&hash2).is_none());
        assert!(cache.find(&hash3).is_some());
        cache.remove(&hash1);
        cache.remove(&hash3);

        assert!(cache.cache_empty());
        assert!(cache.find(&hash1).is_none());
        assert!(cache.find(&hash2).is_none());
        assert!(cache.find(&hash3).is_none());
    }

    /*
     * Ensure that when cache is overfilled, we remove the oldest entries first
     */
    #[test]
    // TODO: takes a long time -> shrink example?
    fn overfill() {
        // Create a vote cache with max size set to 1024
        let mut cache = VoteCache::new(Config { max_size: 1024 });

        let count = 16 * 1024;
        for n in 0..count {
            let weight = count - n;
            let rep1 = create_rep(weight);
            let hash1 = BlockHash::random();
            let vote1 = create_vote(&rep1, &hash1, 1);
            vote(&mut cache, &vote1);
        }

        assert!((cache.cache_size() as u128) < count);
        dbg!(&cache);

        let peek1 = peek(&cache);
        // Check that oldest votes are dropped first
        assert_eq!(peek1.tally, 1024);
    }

    /*
     * Check that when a single vote cache entry is overfilled, it ignores any new votes
     */
    #[test]
    fn overfill_entry() {
        let mut cache = VoteCache::new(Config { max_size: 1024 });
        let count = 1024;

        let hash1 = BlockHash::random();
        for _ in 0..count {
            let rep1 = create_rep(9);
            let vote1 = create_vote(&rep1, &hash1, 1);
            vote(&mut cache, &vote1);
        }
        assert_eq!(cache.cache_size(), 1);
    }
}
