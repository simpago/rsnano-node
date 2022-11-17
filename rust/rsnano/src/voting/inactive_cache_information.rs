use std::time::SystemTime;
use crate::core::{Account, BlockHash};
use crate::voting::InactiveCacheStatus;

pub struct InactiveCacheInformation {
    pub arrival: SystemTime,
    pub hash: BlockHash,
    pub status: InactiveCacheStatus,
    pub voters: Vec<(Account, u64)>,
}