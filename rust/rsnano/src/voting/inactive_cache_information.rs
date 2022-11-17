use crate::core::{Account, BlockHash};
use crate::voting::InactiveCacheStatus;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Information on the status of inactive cache information
#[derive(Clone)]
pub struct InactiveCacheInformation {
    pub arrival: SystemTime,
    pub hash: BlockHash,
    pub status: InactiveCacheStatus,
    pub voters: Vec<(Account, SystemTime)>,
}

impl InactiveCacheInformation {
    pub fn null() -> Self {
        Self {
            arrival: SystemTime::now(),
            hash: BlockHash::default(),
            status: InactiveCacheStatus::default(),
            voters: Vec::new(),
        }
    }

    pub fn new(
        arrival: SystemTime,
        hash: BlockHash,
        status: InactiveCacheStatus,
        initial_rep_a: Account,
        initial_timestamp_a: u64,
    ) -> Self {
        let mut voters = Vec::new();
        voters.push((
            initial_rep_a,
            UNIX_EPOCH
                .checked_add(Duration::from_millis(initial_timestamp_a))
                .unwrap(),
        ));
        Self {
            arrival,
            hash,
            status,
            voters,
        }
    }
}
