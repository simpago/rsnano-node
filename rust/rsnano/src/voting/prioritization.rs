use crate::core::BlockEnum;
use crate::ffi::core::BlockHandle;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use num_format::Locale::ti;

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
