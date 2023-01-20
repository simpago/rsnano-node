use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, Weak,
    },
};

use rsnano_core::BlockHash;

use super::ConfHeightDetails;

pub(crate) struct ImplictReceiveCementedMapping {
    //todo: Remove Mutex
    mapping: HashMap<BlockHash, Weak<Mutex<ConfHeightDetails>>>,
    mapping_size: AtomicUsize,
}

impl ImplictReceiveCementedMapping {
    pub fn new() -> Self {
        Self {
            mapping: HashMap::new(),
            mapping_size: AtomicUsize::new(0),
        }
    }

    pub fn add(&mut self, hash: BlockHash, details: &Arc<Mutex<ConfHeightDetails>>) {
        self.mapping.insert(hash, Arc::downgrade(&details));
        self.mapping_size
            .store(self.mapping.len(), Ordering::Relaxed);
    }

    pub fn get(&self, hash: &BlockHash) -> Option<&Weak<Mutex<ConfHeightDetails>>> {
        self.mapping.get(hash)
    }

    pub fn clear(&mut self) {
        self.mapping.clear();
        self.mapping_size.store(0, Ordering::Relaxed);
    }

    pub fn size_atomic(&self) -> usize {
        self.mapping_size.load(Ordering::Relaxed)
    }
}
