use super::TallyKey;
use crate::stats::{DetailType, StatType, Stats};
#[cfg(test)]
use mock_instant::Instant;
use rsnano_core::{
    utils::{ContainerInfo, ContainerInfoComponent},
    Account, Amount, BlockHash, Vote, VoteCode,
};
#[cfg(not(test))]
use std::time::Instant;
use std::{
    cmp::Ordering,
    collections::{BTreeMap, HashMap},
    fmt::Debug,
    mem::size_of,
    sync::Arc,
    time::Duration,
};

#[derive(Clone, Debug, PartialEq)]
pub struct VoteCacheConfig {
    pub max_size: usize,
    pub max_voters: usize,
    pub age_cutoff: Duration,
}

impl Default for VoteCacheConfig {
    fn default() -> Self {
        Self {
            max_size: 1024 * 64,
            max_voters: 64,
            age_cutoff: Duration::from_millis(15 * 60),
        }
    }
}

///	A container holding votes that do not match any active or recently finished elections.
///	It keeps track of votes in two internal structures: cache and queue
///
///	Cache: Stores votes associated with a particular block hash with a bounded maximum number of votes per hash.
///			When cache size exceeds `max_size` oldest entries are evicted first.
pub struct VoteCache {
    config: VoteCacheConfig,
    cache: CacheEntryCollection,
    next_id: usize,
    last_cleanup: Instant,
    stats: Arc<Stats>,
}

impl VoteCache {
    pub fn new(config: VoteCacheConfig, stats: Arc<Stats>) -> Self {
        VoteCache {
            last_cleanup: Instant::now(),
            config,
            cache: CacheEntryCollection::default(),
            next_id: 0,
            stats,
        }
    }

    /// Adds a new vote to cache
    pub fn insert(
        &mut self,
        vote: &Arc<Vote>,
        rep_weight: Amount,
        results: &HashMap<BlockHash, VoteCode>,
    ) {
        // Results map should be empty or have the same hashes as the vote
        debug_assert!(results.is_empty() || vote.hashes.iter().all(|h| results.contains_key(h)));

        // If results map is empty, insert all hashes (meant for testing)
        if results.is_empty() {
            for hash in &vote.hashes {
                self.insert_impl(vote, hash, rep_weight);
            }
        } else {
            for (hash, code) in results {
                // Cache votes with a corresponding active election (indicated by `vote_code::vote`) in case that election gets dropped
                if matches!(code, VoteCode::Vote | VoteCode::Indeterminate) {
                    self.insert_impl(vote, hash, rep_weight)
                }
            }
        }
    }

    fn insert_impl(&mut self, vote: &Arc<Vote>, hash: &BlockHash, rep_weight: Amount) {
        let cache_entry_exists = self.cache.modify_by_hash(hash, |existing| {
            self.stats.inc(StatType::VoteCache, DetailType::Update);
            existing.vote(vote, rep_weight, self.config.max_voters);
        });

        if !cache_entry_exists {
            self.stats.inc(StatType::VoteCache, DetailType::Insert);
            let id = self.next_id;
            self.next_id += 1;
            let mut cache_entry = CacheEntry::new(id, *hash);
            cache_entry.vote(vote, rep_weight, self.config.max_voters);
            self.cache.insert(cache_entry);

            // Remove the oldest entry if we have reached the capacity limit
            if self.cache.len() > self.config.max_size {
                self.cache.pop_front();
            }
        }
    }

    pub fn empty(&self) -> bool {
        self.cache.is_empty()
    }

    pub fn size(&self) -> usize {
        self.cache.len()
    }

    /// Tries to find an entry associated with block hash
    pub fn find(&self, hash: &BlockHash) -> Vec<Arc<Vote>> {
        self.cache
            .get_by_hash(hash)
            .map(|entry| entry.votes())
            .unwrap_or_default()
    }

    /// Removes an entry associated with block hash, does nothing if entry does not exist
    /// return true if hash existed and was erased, false otherwise
    pub fn erase(&mut self, hash: &BlockHash) -> bool {
        self.cache.remove_by_hash(hash).is_some()
    }

    pub fn clear(&mut self) {
        self.cache.clear()
    }

    /// Returns blocks with highest observed tally, greater than `min_tally`
    /// The blocks are sorted in descending order by final tally, then by tally
    /// @param min_tally minimum tally threshold, entries below with their voting weight
    /// below this will be ignore
    pub fn top(&mut self, min_tally: impl Into<Amount>) -> Vec<TopEntry> {
        let min_tally = min_tally.into();
        self.stats.inc(StatType::VoteCache, DetailType::Top);
        if self.last_cleanup.elapsed() >= self.config.age_cutoff / 2 {
            self.cleanup();
            self.last_cleanup = Instant::now();
        }

        let mut results = Vec::new();
        for entry in self.cache.iter_by_tally_desc() {
            let tally = entry.tally();
            if tally < min_tally {
                break;
            }
            results.push(TopEntry {
                hash: entry.hash,
                tally,
                final_tally: entry.final_tally(),
            })
        }

        // Sort by final tally then by normal tally, descending
        results.sort_by(|a, b| {
            let res = b.final_tally.cmp(&b.final_tally);
            if res == Ordering::Equal {
                b.tally.cmp(&a.tally)
            } else {
                res
            }
        });

        results
    }

    fn cleanup(&mut self) {
        self.stats.inc(StatType::VoteCache, DetailType::Cleanup);
        let to_delete: Vec<_> = self
            .cache
            .iter()
            .filter(|i| i.last_vote.elapsed() >= self.config.age_cutoff)
            .map(|i| i.hash)
            .collect();
        for hash in to_delete {
            self.cache.remove_by_hash(&hash);
        }
    }

    pub fn collect_container_info(&self, name: impl Into<String>) -> ContainerInfoComponent {
        ContainerInfoComponent::Composite(
            name.into(),
            vec![ContainerInfoComponent::Leaf(ContainerInfo {
                name: "cache".to_owned(),
                count: self.size(),
                sizeof_element: size_of::<CacheEntry>(),
            })],
        )
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct TopEntry {
    pub hash: BlockHash,
    pub tally: Amount,
    pub final_tally: Amount,
}

/// Stores votes associated with a single block hash
#[derive(Clone)]
pub struct CacheEntry {
    id: usize,
    pub hash: BlockHash,
    pub voters: OrderedVoters,
    pub last_vote: Instant,
    tally: Amount,
    final_tally: Amount,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoterEntry {
    pub representative: Account,
    pub weight: Amount,
    pub vote: Arc<Vote>,
}

impl VoterEntry {
    pub fn new(representative: Account, weight: Amount, vote: Arc<Vote>) -> Self {
        Self {
            representative,
            weight,
            vote,
        }
    }

    pub fn final_weight(&self) -> Amount {
        if self.vote.is_final() {
            self.weight
        } else {
            Amount::zero()
        }
    }
}

impl CacheEntry {
    pub fn new(id: usize, hash: BlockHash) -> Self {
        CacheEntry {
            id,
            hash,
            voters: OrderedVoters::default(),
            last_vote: Instant::now(),
            tally: Amount::zero(),
            final_tally: Amount::zero(),
        }
    }

    fn calculate_tally(&mut self) -> (Amount, Amount) {
        let mut tally = Amount::zero();
        let mut final_tally = Amount::zero();
        for voter in self.voters.iter_unordered() {
            tally = tally.wrapping_add(voter.weight);
            if voter.vote.is_final() {
                final_tally = final_tally.wrapping_add(voter.weight);
            }
        }
        (tally, final_tally)
    }

    pub fn tally(&self) -> Amount {
        self.tally
    }

    pub fn final_tally(&self) -> Amount {
        self.final_tally
    }

    pub fn votes(&self) -> Vec<Arc<Vote>> {
        self.voters
            .iter_unordered()
            .map(|i| Arc::clone(&i.vote))
            .collect()
    }

    /// Adds a vote into a list, checks for duplicates and updates timestamp if new one is greater
    /// returns true if current tally changed, false otherwise
    pub fn vote(&mut self, vote: &Arc<Vote>, rep_weight: Amount, max_voters: usize) -> bool {
        let updated = self.vote_impl(vote, rep_weight, max_voters);
        if updated {
            (self.tally, self.final_tally) = self.calculate_tally();
            self.last_vote = Instant::now();
        }
        updated
    }

    fn vote_impl(&mut self, vote: &Arc<Vote>, rep_weight: Amount, max_voters: usize) -> bool {
        let representative = vote.voting_account;

        if let Some(existing) = self.voters.find(&representative) {
            // We already have a vote from this rep
            // Update timestamp if newer but tally remains unchanged as we already counted this rep weight
            // It is not essential to keep tally up to date if rep voting weight changes, elections do tally calculations independently, so in the worst case scenario only our queue ordering will be a bit off
            if vote.timestamp() > existing.vote.timestamp() {
                let was_final = existing.vote.is_final();
                self.voters
                    .modify(&representative, Arc::clone(vote), rep_weight);
                return !was_final && vote.is_final(); // Tally changed only if the vote became final
            } else {
                return false;
            }
        }

        let should_add = if self.voters.len() < max_voters {
            true
        } else {
            let min_weight = self.voters.min_weight().expect("voters must not be empty");
            rep_weight > min_weight
        };

        // Vote from a new representative, add it to the list and update tally
        if should_add {
            self.voters.insert(VoterEntry::new(
                representative,
                rep_weight,
                Arc::clone(&vote),
            ));

            // If we have reached the maximum number of voters, remove the lowest weight voter
            if self.voters.len() >= max_voters {
                self.voters.remove_lowest_weight();
            }
            return true;
        }
        false
    }

    pub fn size(&self) -> usize {
        self.voters.len()
    }
}

#[derive(Default)]
pub struct CacheEntryCollection {
    sequential: BTreeMap<usize, BlockHash>,
    by_hash: HashMap<BlockHash, CacheEntry>,
    by_tally: BTreeMap<TallyKey, Vec<BlockHash>>,
}

impl CacheEntryCollection {
    pub fn insert(&mut self, entry: CacheEntry) {
        let old = self.sequential.insert(entry.id, entry.hash);
        debug_assert!(old.is_none());

        let tally = entry.tally().into();
        self.by_tally.entry(tally).or_default().push(entry.hash);

        let old = self.by_hash.insert(entry.hash, entry);
        debug_assert!(old.is_none());
    }

    pub fn modify_by_hash<F>(&mut self, hash: &BlockHash, f: F) -> bool
    where
        F: FnOnce(&mut CacheEntry),
    {
        if let Some(entry) = self.by_hash.get_mut(hash) {
            let old_tally = entry.tally();
            f(entry);
            let new_tally = entry.tally();
            let hash = entry.hash;
            self.update_tally(hash, old_tally, new_tally);
            true
        } else {
            false
        }
    }

    fn update_tally(&mut self, hash: BlockHash, old_tally: Amount, new_tally: Amount) {
        if old_tally == new_tally {
            return;
        }
        self.remove_by_tally(hash, old_tally);
        self.by_tally
            .entry(new_tally.into())
            .or_default()
            .push(hash);
    }

    fn remove_by_tally(&mut self, hash: BlockHash, tally: Amount) {
        let key = TallyKey::from(tally);
        let hashes = self.by_tally.get_mut(&key).unwrap();
        if hashes.len() == 1 {
            self.by_tally.remove(&key);
        } else {
            hashes.retain(|h| *h != hash)
        }
    }

    pub fn pop_front(&mut self) -> Option<CacheEntry> {
        match self.sequential.pop_first() {
            Some((_, front_hash)) => {
                let entry = self.by_hash.remove(&front_hash).unwrap();
                self.remove_by_tally(front_hash, entry.tally());
                Some(entry)
            }
            None => None,
        }
    }

    pub fn get_by_hash(&self, hash: &BlockHash) -> Option<&CacheEntry> {
        self.by_hash.get(hash)
    }

    pub fn remove_by_hash(&mut self, hash: &BlockHash) -> Option<CacheEntry> {
        match self.by_hash.remove(hash) {
            Some(entry) => {
                self.sequential.remove(&entry.id);
                self.remove_by_tally(*hash, entry.tally());
                Some(entry)
            }
            None => None,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &CacheEntry> {
        self.by_hash.values()
    }

    pub fn iter_by_tally_desc(&self) -> impl Iterator<Item = &CacheEntry> {
        self.by_tally
            .values()
            .flat_map(|hashes| hashes.iter().map(|hash| self.by_hash.get(hash).unwrap()))
    }

    pub fn len(&self) -> usize {
        self.sequential.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sequential.is_empty()
    }

    pub fn clear(&mut self) {
        self.sequential.clear();
        self.by_hash.clear();
        self.by_tally.clear();
    }
}

#[derive(Default, Clone)]
pub struct OrderedVoters {
    by_representative: HashMap<Account, VoterEntry>,
    by_weight: BTreeMap<Amount, Vec<Account>>,
}

impl OrderedVoters {
    pub fn insert(&mut self, entry: VoterEntry) {
        let weight = entry.weight;
        let rep = entry.representative;
        if let Some(existing) = self.by_representative.get_mut(&rep) {
            let old_weight = existing.weight;
            *existing = entry;
            self.remove_by_weight(&old_weight, &rep);
        } else {
            self.by_representative.insert(rep, entry);
        }
        self.add_by_weight(weight, rep);
    }

    pub fn iter_unordered(&self) -> impl Iterator<Item = &VoterEntry> {
        self.by_representative.values()
    }

    pub fn find(&self, representative: &Account) -> Option<&VoterEntry> {
        self.by_representative.get(representative)
    }

    pub fn first(&self) -> Option<&VoterEntry> {
        self.by_weight
            .first_key_value()
            .and_then(|(_, reps)| reps.first())
            .and_then(|rep| self.by_representative.get(rep))
    }

    pub fn modify(&mut self, representative: &Account, vote: Arc<Vote>, new_weight: Amount) {
        if let Some(entry) = self.by_representative.get_mut(representative) {
            let old_weight = entry.weight;
            entry.vote = vote;
            entry.weight = new_weight;
            if old_weight != new_weight {
                self.remove_by_weight(&old_weight, representative);
                self.add_by_weight(new_weight, *representative);
            }
        }
    }

    pub fn min_weight(&self) -> Option<Amount> {
        self.by_weight
            .first_key_value()
            .map(|(weight, _reps)| *weight)
    }

    pub fn remove_lowest_weight(&mut self) {
        if let Some((_, reps)) = self.by_weight.pop_first() {
            for rep in reps {
                self.by_representative.remove(&rep);
            }
        }
    }

    pub fn len(&self) -> usize {
        self.by_representative.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_representative.is_empty()
    }

    fn remove_by_weight(&mut self, weight: &Amount, representative: &Account) {
        if let Some(mut accounts) = self.by_weight.remove(weight) {
            if accounts.len() > 1 {
                accounts.retain(|a| a != representative);
                self.by_weight.insert(*weight, accounts);
            }
        }
    }

    fn add_by_weight(&mut self, weight: Amount, representative: Account) {
        self.by_weight
            .entry(weight)
            .or_default()
            .push(representative);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::Direction;
    use mock_instant::MockClock;
    use rsnano_core::KeyPair;

    fn create_vote(rep: &KeyPair, hash: &BlockHash, timestamp_offset: u64) -> Arc<Vote> {
        Arc::new(Vote::new(
            rep.public_key(),
            &rep.private_key(),
            timestamp_offset * 1024 * 1024,
            0,
            vec![*hash],
        ))
    }

    fn create_final_vote(rep: &KeyPair, hash: &BlockHash) -> Arc<Vote> {
        Arc::new(Vote::new_final(rep, vec![*hash]))
    }

    fn test_config() -> VoteCacheConfig {
        VoteCacheConfig {
            max_size: 3,
            max_voters: 80,
            age_cutoff: Duration::from_secs(5 * 60),
        }
    }

    fn create_vote_cache() -> VoteCache {
        VoteCache::new(test_config(), Arc::new(Stats::new(Default::default())))
    }

    #[test]
    fn construction() {
        let cache = create_vote_cache();
        assert_eq!(cache.size(), 0);
        assert!(cache.empty());
        let hash = BlockHash::random();
        assert!(cache.find(&hash).is_empty());
    }

    #[test]
    fn insert_one_hash() {
        let mut cache = create_vote_cache();
        let rep = KeyPair::new();
        let hash = BlockHash::from(1);
        let vote = create_vote(&rep, &hash, 1);

        cache.insert(&vote, Amount::raw(7), &HashMap::new());

        assert_eq!(cache.size(), 1);
        let peek = cache.find(&hash);
        assert_eq!(peek.len(), 1);
        assert_eq!(peek.first(), Some(&vote));
    }

    /*
     * Inserts multiple votes for single hash
     * Ensures all of them can be retrieved and that tally is properly accumulated
     */
    #[test]
    fn insert_one_hash_many_votes() {
        let mut cache = create_vote_cache();

        let hash = BlockHash::random();
        let rep1 = KeyPair::new();
        let rep2 = KeyPair::new();
        let rep3 = KeyPair::new();

        let vote1 = create_vote(&rep1, &hash, 1);
        let vote2 = create_vote(&rep2, &hash, 2);
        let vote3 = create_vote(&rep3, &hash, 3);

        cache.insert(&vote1, Amount::raw(7), &HashMap::new());
        cache.insert(&vote2, Amount::raw(9), &HashMap::new());
        cache.insert(&vote3, Amount::raw(11), &HashMap::new());
        // We have 3 votes but for a single hash, so just one entry in vote cache
        assert_eq!(cache.size(), 1);
        let votes = cache.find(&hash);
        assert_eq!(votes.len(), 3);
    }

    #[test]
    fn insert_many_hashes_many_votes() {
        let mut cache = create_vote_cache();

        // There will be 3 hashes to vote for
        let hash1 = BlockHash::from(1);
        let hash2 = BlockHash::from(2);
        let hash3 = BlockHash::from(3);

        // There will be 4 reps with different weights
        let rep1 = KeyPair::new();
        let rep2 = KeyPair::new();
        let rep3 = KeyPair::new();
        let rep4 = KeyPair::new();

        // Votes: rep1 > hash1, rep2 > hash2, rep3 > hash3, rep4 > hash1 (the same as rep1)
        let vote1 = create_vote(&rep1, &hash1, 1);
        let vote2 = create_vote(&rep2, &hash2, 1);
        let vote3 = create_vote(&rep3, &hash3, 1);
        let vote4 = create_vote(&rep4, &hash1, 1);

        // Insert first 3 votes in cache
        cache.insert(&vote1, Amount::raw(7), &HashMap::new());
        cache.insert(&vote2, Amount::raw(9), &HashMap::new());
        cache.insert(&vote3, Amount::raw(11), &HashMap::new());

        // Ensure all of those are properly inserted
        assert_eq!(cache.size(), 3);
        assert_eq!(cache.find(&hash1).len(), 1);
        assert_eq!(cache.find(&hash2).len(), 1);
        assert_eq!(cache.find(&hash3).len(), 1);

        // Now add a vote from rep4 with the highest voting weight
        cache.insert(&vote4, Amount::raw(13), &HashMap::new());

        let pop1 = cache.find(&hash1);
        assert_eq!(pop1.len(), 2);

        let pop2 = cache.find(&hash3);
        assert_eq!(pop2.len(), 1);
    }

    /*
     * Ensure that duplicate votes are ignored
     */
    #[test]
    fn insert_duplicate() {
        let mut cache = create_vote_cache();

        let hash = BlockHash::from(1);
        let rep = KeyPair::new();
        let vote1 = create_vote(&rep, &hash, 1);
        let vote2 = create_vote(&rep, &hash, 1);

        cache.insert(&vote1, Amount::raw(9), &HashMap::new());
        cache.insert(&vote2, Amount::raw(9), &HashMap::new());

        assert_eq!(cache.size(), 1)
    }

    /*
     * Ensure that when processing vote from a representative that is already cached, we always update to the vote with the highest timestamp
     */
    #[test]
    fn insert_newer() {
        let mut cache = create_vote_cache();

        let hash = BlockHash::from(1);
        let rep = KeyPair::new();
        let vote1 = create_vote(&rep, &hash, 1);
        cache.insert(&vote1, Amount::raw(9), &HashMap::new());

        let vote2 = Arc::new(Vote::new(
            rep.public_key(),
            &rep.private_key(),
            Vote::TIMESTAMP_MAX,
            Vote::DURATION_MAX,
            vec![hash],
        ));
        cache.insert(&vote2, Amount::raw(9), &HashMap::new());

        let peek2 = cache.find(&hash);
        assert_eq!(peek2.len(), 1);
        assert_eq!(peek2.first().unwrap().timestamp(), Vote::FINAL_TIMESTAMP); // final timestamp
    }

    /*
     * Ensure that when processing vote from a representative that is already cached, votes with older timestamp are ignored
     */
    #[test]
    fn insert_older() {
        let mut cache = create_vote_cache();
        let hash = BlockHash::from(1);
        let rep = KeyPair::new();
        let vote1 = create_vote(&rep, &hash, 2);
        cache.insert(&vote1, Amount::raw(9), &HashMap::new());
        let peek1 = cache.find(&hash);

        let vote2 = create_vote(&rep, &hash, 1);
        cache.insert(&vote2, Amount::raw(9), &HashMap::new());
        let peek2 = cache.find(&hash);

        assert_eq!(cache.size(), 1);
        assert_eq!(peek2.len(), 1);
        assert_eq!(
            peek2.first().unwrap().timestamp(),
            peek1.first().unwrap().timestamp()
        ); // timestamp2 == timestamp1
    }

    /*
     * Ensure that erase functionality works
     */
    #[test]
    fn erase() {
        let mut cache = create_vote_cache();
        let hash1 = BlockHash::from(1);
        let hash2 = BlockHash::from(2);
        let hash3 = BlockHash::from(3);

        let rep1 = KeyPair::new();
        let rep2 = KeyPair::new();
        let rep3 = KeyPair::new();

        let vote1 = create_vote(&rep1, &hash1, 1);
        let vote2 = create_vote(&rep2, &hash2, 1);
        let vote3 = create_vote(&rep3, &hash3, 1);

        cache.insert(&vote1, Amount::raw(7), &HashMap::new());
        cache.insert(&vote2, Amount::raw(9), &HashMap::new());
        cache.insert(&vote3, Amount::raw(11), &HashMap::new());

        assert_eq!(cache.size(), 3);
        assert_eq!(cache.find(&hash1).len(), 1);
        assert_eq!(cache.find(&hash2).len(), 1);
        assert_eq!(cache.find(&hash3).len(), 1);

        cache.erase(&hash2);

        assert_eq!(cache.size(), 2);
        assert_eq!(cache.find(&hash1).len(), 1);
        assert_eq!(cache.find(&hash2).len(), 0);
        assert_eq!(cache.find(&hash3).len(), 1);
        cache.erase(&hash1);
        cache.erase(&hash3);

        assert!(cache.empty());
    }

    /*
     * Ensure that when cache is overfilled, we remove the oldest entries first
     */
    #[test]
    fn overfill() {
        let mut cache = create_vote_cache();

        let hash1 = BlockHash::from(1);
        let hash2 = BlockHash::from(2);
        let hash3 = BlockHash::from(3);
        let hash4 = BlockHash::from(4);

        let rep1 = KeyPair::new();
        let rep2 = KeyPair::new();
        let rep3 = KeyPair::new();
        let rep4 = KeyPair::new();

        let vote1 = create_vote(&rep1, &hash1, 1);
        cache.insert(&vote1, Amount::raw(1), &HashMap::new());

        let vote2 = create_vote(&rep2, &hash2, 1);
        cache.insert(&vote2, Amount::raw(2), &HashMap::new());

        let vote3 = create_vote(&rep3, &hash3, 1);
        cache.insert(&vote3, Amount::raw(3), &HashMap::new());

        let vote4 = create_vote(&rep4, &hash4, 1);
        cache.insert(&vote4, Amount::raw(4), &HashMap::new());

        assert_eq!(cache.size(), 3);

        // Check that oldest votes are dropped first
        assert_eq!(cache.find(&hash4).len(), 1);
        assert_eq!(cache.find(&hash3).len(), 1);
        assert_eq!(cache.find(&hash2).len(), 1);
        assert_eq!(cache.find(&hash1).len(), 0);
    }

    /*
     * Check that when a single vote cache entry is overfilled, it ignores any new votes
     */
    #[test]
    fn overfill_entry() {
        let mut cache = create_vote_cache();
        let hash = BlockHash::from(1);

        let rep1 = KeyPair::new();
        let vote1 = create_vote(&rep1, &hash, 1);
        cache.insert(&vote1, Amount::raw(9), &HashMap::new());

        let rep2 = KeyPair::new();
        let vote2 = create_vote(&rep2, &hash, 1);
        cache.insert(&vote2, Amount::raw(9), &HashMap::new());

        let rep3 = KeyPair::new();
        let vote3 = create_vote(&rep3, &hash, 1);
        cache.insert(&vote3, Amount::raw(9), &HashMap::new());

        assert_eq!(cache.size(), 1);
    }

    #[test]
    fn change_vote_to_final_vote() {
        let mut cache = create_vote_cache();
        let hash = BlockHash::from(1);

        let rep = KeyPair::new();
        let vote = create_vote(&rep, &hash, 1);
        let final_vote = create_final_vote(&rep, &hash);
        cache.insert(&vote, Amount::raw(9), &HashMap::new());
        cache.insert(&final_vote, Amount::raw(9), &HashMap::new());

        let votes = cache.find(&hash);
        let vote = votes.first().unwrap();
        assert!(vote.is_final());
    }

    #[test]
    fn add_final_vote() {
        let mut cache = create_vote_cache();
        let hash = BlockHash::from(1);

        let rep = KeyPair::new();
        let vote = create_final_vote(&rep, &hash);
        cache.insert(&vote, Amount::raw(9), &HashMap::new());

        let votes = cache.find(&hash);
        let vote = votes.first().unwrap();
        assert!(vote.is_final());
    }

    #[test]
    fn top_empty() {
        let mut cache = create_vote_cache();
        assert_eq!(cache.top(0), Vec::new());
    }

    #[test]
    fn top_one_entry() {
        let mut cache = create_vote_cache();
        let hash = BlockHash::from(1);
        add_test_vote(&mut cache, &hash, Amount::raw(1));

        assert_eq!(
            cache.top(0),
            vec![TopEntry {
                hash,
                tally: Amount::raw(1),
                final_tally: Amount::zero()
            }]
        );
    }

    #[test]
    fn top_multiple_entries_sorted_by_tally() {
        let mut cache = create_vote_cache();
        let hash1 = BlockHash::from(1);
        let hash2 = BlockHash::from(2);
        let hash3 = BlockHash::from(3);
        add_test_vote(&mut cache, &hash1, Amount::raw(1));
        add_test_vote(&mut cache, &hash2, Amount::raw(4));
        add_test_vote(&mut cache, &hash3, Amount::raw(3));
        add_test_final_vote(&mut cache, &hash2, Amount::raw(5));
        add_test_final_vote(&mut cache, &hash3, Amount::raw(5));

        let top = cache.top(0);

        assert_eq!(top.len(), 3);
        assert_eq!(top[0].hash, hash2);
        assert_eq!(top[1].hash, hash3);
        assert_eq!(top[2].hash, hash1);
    }

    #[test]
    fn top_min_tally() {
        let mut cache = create_vote_cache();
        let hash1 = BlockHash::from(1);
        let hash2 = BlockHash::from(2);
        let hash3 = BlockHash::from(3);
        add_test_vote(&mut cache, &hash1, Amount::raw(1));
        add_test_vote(&mut cache, &hash2, Amount::raw(2));
        add_test_vote(&mut cache, &hash3, Amount::raw(3));

        let top = cache.top(2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].hash, hash3);
        assert_eq!(top[1].hash, hash2);
    }

    #[test]
    fn top_age_cutoff() {
        let stats = Arc::new(Stats::new(Default::default()));
        let mut cache = VoteCache::new(test_config(), Arc::clone(&stats));
        let hash = BlockHash::from(1);
        add_test_vote(&mut cache, &hash, Amount::raw(1));
        assert_eq!(
            stats.count(StatType::VoteCache, DetailType::Cleanup, Direction::In),
            0
        );
        MockClock::advance(Duration::from_secs(150));
        assert_eq!(cache.top(0).len(), 1);
        assert_eq!(
            stats.count(StatType::VoteCache, DetailType::Cleanup, Direction::In),
            1
        );
        MockClock::advance(Duration::from_secs(150));
        assert_eq!(cache.top(0).len(), 0);
        assert_eq!(
            stats.count(StatType::VoteCache, DetailType::Cleanup, Direction::In),
            2
        );
    }

    fn add_test_vote(cache: &mut VoteCache, hash: &BlockHash, rep_weight: Amount) {
        let vote = create_vote(&KeyPair::new(), &hash, 0);
        cache.insert(&vote, rep_weight, &HashMap::new());
    }

    fn add_test_final_vote(cache: &mut VoteCache, hash: &BlockHash, rep_weight: Amount) {
        let vote = create_final_vote(&KeyPair::new(), &hash);
        cache.insert(&vote, rep_weight, &HashMap::new());
    }
}
