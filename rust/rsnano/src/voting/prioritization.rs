use crate::core::{Amount, Block, BlockEnum, BlockType, SendBlock};
use crate::ffi::core::BlockHandle;
use crate::ledger::datastore::lmdb::get;
use num_format::Locale::{el, se, ti};
use std::cmp::{max, Ordering};
use std::collections::BTreeSet;
use std::ops::Deref;
use std::ptr::null;
use std::sync::{Arc, RwLock};
use std::thread::current;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use toml_edit::value;

/// Information on the value type
#[derive(Clone, Debug)]
pub struct ValueType {
    time: Option<SystemTime>,
    block: Option<Arc<RwLock<BlockEnum>>>,
}

impl Ord for ValueType {
    fn cmp(&self, other: &Self) -> Ordering {
        /*let t1 = self.time.unwrap();
        let t2 = &other.time.unwrap();
        let b1 = self.block.as_ref().unwrap().read().unwrap().clone();
        let b2 = other.block.as_ref().unwrap().read().unwrap().clone();
        if t1 != *t2 {
            t1.cmp(t2)
        }
        else {
            b1.as_block().hash().number().cmp(&b2.as_block().hash().number())
        }*/
        self.time.unwrap().cmp(&other.time.unwrap())
    }
}

impl PartialOrd for ValueType {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for ValueType {
    fn eq(&self, other: &Self) -> bool {
        let b1 = self.block.as_ref().unwrap().read().unwrap().clone();
        let b2 = other.block.as_ref().unwrap().read().unwrap().clone();
        b1.as_block().hash().number().eq(&b2.as_block().hash().number())
    }
}

impl Eq for ValueType {}

impl ValueType {
    pub fn new(time: u64, block: Option<Arc<RwLock<BlockEnum>>>) -> Self {
        Self {
            time: UNIX_EPOCH.checked_add(Duration::from_millis(time)),
            block,
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
        let mut minimums: [u128; BUCKET_COUNT] = [0; BUCKET_COUNT];
        let mut value = 1;
        let mut buckets = [(); BUCKET_COUNT].map(|_| BTreeSet::new());
        let mut schedule: [u8; BUCKET_COUNT] = [0; BUCKET_COUNT];
        minimums[0] = 0;

        for i in 1..BUCKET_COUNT {
            minimums[i] = value;
            value = value << 1;
        }

        for i in 0..buckets.len() {
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
            size += self.bucket_size(i);
        }
        size
    }

    /// Moves the bucket pointer to the next bucket
    pub fn next(&mut self) {
        if self.current < BUCKET_COUNT - 1 {
            self.current += 1;
        } else {
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
        self.next();
        for _ in 0..self.schedule.len() {
            if self.buckets[self.current].is_empty() {
                self.next();
            }
        }
        println!("Current: {}", self.current);
    }

    /// Return the highest priority block of the current bucket
    pub fn top(&mut self) -> Option<Arc<RwLock<BlockEnum>>> {
        debug_assert!(!self.empty());
        debug_assert!(!self.buckets[self.current].is_empty());
        match self.buckets[self.current].first() {
            Some(b) => b.block.clone(),
            None => None,
        }
    }

    /// Push a block and its associated time into the prioritization container.
    /// The time is given here because sideband might not exist in the case of state blocks.
    pub fn push(&mut self, time: u64, block: Arc<RwLock<BlockEnum>>) {
        let was_empty = self.empty();
        let binding = block.read().unwrap();
        let block_enum = binding.deref().clone();
        let block_has_balance = block_enum.block_type() == BlockType::State || block_enum.block_type() == BlockType::Send;
        debug_assert!(block_has_balance || block_enum.sideband().is_some());
        let mut balance = Amount::zero();
        match block_enum {
            BlockEnum::Send(b) => balance = b.balance(),
            BlockEnum::State(b) => balance = b.balance(),
            BlockEnum::Open(b) => balance = b.sideband().unwrap().balance,
            BlockEnum::Change(b) => balance = b.sideband().unwrap().balance,
            BlockEnum::Receive(b) => balance = b.sideband().unwrap().balance,
        }
        let mut index: usize = 0;
        for i in 0..self.minimums.len() {
            if balance.number() < self.minimums[i] {
                index = i as usize - 1 - self.minimums[0] as usize;
                break;
            }
        }
        self.buckets[index].insert(ValueType::new(time, Some(block.clone())));
        if self.buckets[index].len() > max(1, (self.maximum / self.buckets.len() as u64) as usize) {
            self.buckets[index].pop_last();
        }
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
        let mut result = true;
        for i in 0..BUCKET_COUNT {
            if !self.buckets[i].is_empty() {
                return false;
            }
        }
        result
    }
}
