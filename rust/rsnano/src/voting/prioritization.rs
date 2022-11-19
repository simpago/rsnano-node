use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::ops::Deref;
use std::ptr::null;
use crate::core::{Amount, Block, BlockEnum, BlockType, SendBlock};
use crate::ffi::core::BlockHandle;
use std::sync::{Arc, RwLock};
use std::thread::current;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use num_format::Locale::{el, se, ti};
use toml_edit::value;

/// Information on the value type
#[derive(Clone, Default, Debug)]
pub struct ValueType {
    time: Option<SystemTime>,
    block: Option<Arc<RwLock<BlockEnum>>>,
}

impl Ord for ValueType {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.time).cmp(&(other.time))
    }
}

impl PartialOrd for ValueType {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for ValueType {
    fn eq(&self, other: &Self) -> bool {
        ((*self.block.as_ref().unwrap()).as_ref().clone().read().unwrap().deref()) == ((*other.block.as_ref().unwrap()).as_ref().clone().read().unwrap().deref())
    }
}

impl Eq for ValueType { }

impl ValueType {
    pub fn new(time: u64, block: Option<Arc<RwLock<BlockEnum>>>) -> Self {
        Self {
            time: UNIX_EPOCH.checked_add(Duration::from_millis(time)),
            block
        }
    }

    pub fn get_time(&self) -> u64 {
        self.time
            .map(|x| x.duration_since(UNIX_EPOCH).unwrap().as_millis() as u64)
            .unwrap_or_default()
    }

    pub fn get_block(&self) -> Option<Arc<RwLock<BlockEnum>>> {
        self.block.clone()
    }
}

const BUCKET_COUNT: usize = 129;

#[derive(Clone, Debug)]
pub struct Prioritization {
    buckets: [BTreeSet<ValueType>; BUCKET_COUNT],
    minimums: [u128; BUCKET_COUNT],
    schedule: [u8; BUCKET_COUNT],
    current: usize,
    maximum: u64,
}

impl Prioritization {
    pub fn new(maximum: u64) -> Self {
        let mut minimums: [u128; BUCKET_COUNT]= [0; BUCKET_COUNT];
        let mut value = 1;
        let mut buckets = [(); BUCKET_COUNT].map(|_| BTreeSet::new());;
        let mut schedule: [u8; BUCKET_COUNT] = [0; BUCKET_COUNT];
        
        for i in 0..BUCKET_COUNT {
            minimums[i] = value;
            value << 1;
            schedule[i] = i as u8;
        }
        
        Self {
            buckets,
            minimums,
            schedule,
            current: 0,
            maximum,
        }
    }

    /// Returns the total number of blocks in buckets
    pub fn size(&self) -> usize {
        let mut size = 0;
        for i in 0..self.buckets.len() {
            size += self.buckets[i].len()
        }
        size
    }

    /// Moves the bucket pointer to the next bucket
    pub fn next(&mut self) {
        if self.current < BUCKET_COUNT {
            self.current += 1;
        }
        else {
            self.current = 0;
        }
    }

    /// Pop the current block from the container and seek to the next block, if it exists
    pub fn pop(&mut self) {
        self.buckets[self.current].pop_first();
        self.seek();
    }

    /// Seek to the next non-empty bucket, if one exists
    pub fn seek(&mut self) {
        for i in 0..BUCKET_COUNT {
            self.next();
            if self.buckets[self.current].is_empty() {
                self.next();
            }
        }
    }

    /// Return the highest priority block of the current bucket
    pub fn top(&mut self) -> Option<Arc<RwLock<BlockEnum>>> {
        match self.buckets[self.current].pop_first() {
            Some(b) => b.block,
            None => None
        }
    }

    /// Push a block and its associated time into the prioritization container.
    /// The time is given here because sideband might not exist in the case of state blocks.
    pub fn push(&mut self, time: u64, block: Arc<RwLock<BlockEnum>>) {
        /*auto was_empty = empty ();
        auto block_has_balance = block->type () == nano::block_type::state || block->type () == nano::block_type::send;
        debug_assert (block_has_balance || block->has_sideband ());
        auto balance = block_has_balance ? block->balance () : block->sideband ().balance ();
        auto index = std::upper_bound (minimums.begin (), minimums.end (), balance.number ()) - 1 - minimums.begin ();
        auto & bucket = buckets[index];
        bucket.emplace (value_type{ time, block });
        if (bucket.size () > std::max (decltype (maximum){ 1 }, maximum / buckets.size ()))
        {
            bucket.erase (--bucket.end ());
        }
        if (was_empty)
        {
            seek ();
        }*/
        let was_empty = self.empty();
        let binding = block.read().unwrap();
        let block_enum = binding.deref().clone();
        let block_has_balance = block_enum.block_type() == BlockType::State || block_enum.block_type() == BlockType::Send;
        debug_assert!(block_has_balance || block_enum.sideband().is_some());
        let mut balance = Amount::zero();
        match block_enum {
            BlockEnum::Send(b) => balance = b.balance(),
            BlockEnum::State(b) => balance = b.balance(),
            BlockEnum::Open(b) => balance = b.balance(),
            BlockEnum::Change(b) => balance = b.balance(),
            BlockEnum::Receive(b) => balance = b.balance(),
        }
        let index = 0;
        //auto index = std::upper_bound (minimums.begin (), minimums.end (), balance.number ()) - 1 - minimums.begin ();
        self.buckets[index].insert(ValueType::new(time, Some(block.clone())));
        /*if (bucket.size () > std::max (decltype (maximum){ 1 }, maximum / buckets.size ()))
        {
            bucket.erase (--bucket.end ());
        }*/
        if was_empty {
            self.seek();
        }
    }

    /// Returns number of buckets, 129 by default
    pub fn bucket_count(&self) -> usize {
        self.buckets.len()
    }

    /// Returns number of items in bucket with index 'index'
    pub fn bucket_size(&self, index: usize) -> usize {
        self.buckets[index].len()
    }

    /// Returns true if all buckets are empty
    pub fn empty(&self) -> bool {
        let mut result = false;
        for i in 0..BUCKET_COUNT {
            if !self.buckets[i].is_empty() {
                return result;
            }
        }
        result = true;
        result
    }

    /// Print the state of the class
    pub fn dump() {

    }
}