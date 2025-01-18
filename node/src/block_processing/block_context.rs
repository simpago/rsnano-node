use crate::stats::DetailType;
use rsnano_core::{Block, SavedBlock};
use rsnano_ledger::BlockStatus;
use std::{
    sync::{Arc, Condvar, Mutex},
    time::Instant,
};
use strum_macros::EnumIter;

#[derive(FromPrimitive, Copy, Clone, PartialEq, Eq, Debug, PartialOrd, Ord, EnumIter, Hash)]
pub enum BlockSource {
    Unknown = 0,
    Live,
    LiveOriginator,
    Bootstrap,
    BootstrapLegacy,
    Unchecked,
    Local,
    Forced,
    Election,
}

impl From<BlockSource> for DetailType {
    fn from(value: BlockSource) -> Self {
        match value {
            BlockSource::Unknown => DetailType::Unknown,
            BlockSource::Live => DetailType::Live,
            BlockSource::LiveOriginator => DetailType::LiveOriginator,
            BlockSource::Bootstrap => DetailType::Bootstrap,
            BlockSource::BootstrapLegacy => DetailType::BootstrapLegacy,
            BlockSource::Unchecked => DetailType::Unchecked,
            BlockSource::Local => DetailType::Local,
            BlockSource::Forced => DetailType::Forced,
            BlockSource::Election => DetailType::Election,
        }
    }
}

pub type BlockProcessorCallback = Box<dyn Fn(BlockStatus) + Send + Sync>;

pub struct BlockContext {
    pub block: Mutex<Block>,
    pub saved_block: Mutex<Option<SavedBlock>>,
    pub source: BlockSource,
    pub callback: Option<BlockProcessorCallback>,
    pub arrival: Instant,
    pub waiter: Arc<BlockProcessorWaiter>,
}

impl BlockContext {
    pub fn new(
        block: Block,
        source: BlockSource,
        callback: Option<BlockProcessorCallback>,
    ) -> Self {
        Self {
            block: Mutex::new(block),
            saved_block: Mutex::new(None),
            source,
            arrival: Instant::now(),
            callback,
            waiter: Arc::new(BlockProcessorWaiter::new()),
        }
    }

    pub fn set_result(&self, result: BlockStatus) {
        self.waiter.set_result(result);
    }

    pub fn get_waiter(&self) -> Arc<BlockProcessorWaiter> {
        self.waiter.clone()
    }
}

impl Drop for BlockContext {
    fn drop(&mut self) {
        self.waiter.cancel()
    }
}

pub struct BlockProcessorWaiter {
    result: Mutex<(Option<BlockStatus>, bool)>, // (status, done)
    condition: Condvar,
}

impl BlockProcessorWaiter {
    pub fn new() -> Self {
        Self {
            result: Mutex::new((None, false)),
            condition: Condvar::new(),
        }
    }

    pub fn set_result(&self, result: BlockStatus) {
        *self.result.lock().unwrap() = (Some(result), true);
        self.condition.notify_all();
    }

    pub fn cancel(&self) {
        self.result.lock().unwrap().1 = true;
        self.condition.notify_all();
    }

    pub fn wait_result(&self) -> Option<BlockStatus> {
        let guard = self.result.lock().unwrap();
        if guard.1 {
            return guard.0;
        }

        self.condition.wait_while(guard, |i| !i.1).unwrap().0
    }
}
