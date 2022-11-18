use std::collections::BTreeSet;
use std::ptr::null;
use crate::core::BlockEnum;
use crate::ffi::core::BlockHandle;
use std::sync::{Arc, RwLock};
use std::thread::current;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use num_format::Locale::{el, se, ti};
use toml_edit::value;

/// Information on the value type
#[derive(Clone, Default)]
pub struct ValueType {
    time: Option<SystemTime>,
    block: Option<Arc<RwLock<BlockEnum>>>,
}

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
    pub fn pop() {

    }

    /// Seek to the next non-empty bucket, if one exists
    pub fn seek() {

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
    pub fn push() {

    }

    /// Returns number of buckets, 129 by default
    pub fn bucket_count() {

    }

    /// Returns number of items in bucket with index 'index'
    pub fn bucket_size() {

    }

    /// Returns true if all buckets are empty
    pub fn empty() {

    }

    /// Print the state of the class
    pub fn dump() {

    }
}