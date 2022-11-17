use crate::core::{Account, Amount};
use crate::voting::ElectionStatus;
use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

pub struct RecentlyCementedCache {
    pub(crate) cemented: Mutex<VecDeque<ElectionStatus>>,
    pub(crate) max_size: usize,
}

impl RecentlyCementedCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            cemented: Mutex::new(VecDeque::new()),
            max_size,
        }
    }

    pub fn get_cemented(&self) -> VecDeque<ElectionStatus> {
        self.cemented.lock().unwrap().clone()
    }
}
