use std::fmt;
use std::fmt::write;
use std::hash::Hash;
use crate::core::{Account, BlockHash};
use crate::voting::InactiveCacheStatus;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use crate::stats::Direction::In;

/// Information on the status of inactive cache information
#[derive(Clone)]
pub struct InactiveCacheInformation {
    pub arrival: Instant,
    pub hash: BlockHash,
    pub status: InactiveCacheStatus,
    pub voters: Vec<(Account, u64)>,
}

impl InactiveCacheInformation {
    pub fn null() -> Self {
        Self {
            arrival: Instant::checked_sub(&Instant::now(), Instant::now().elapsed()).unwrap(),
            hash: BlockHash::default(),
            status: InactiveCacheStatus::default(),
            voters: Vec::new(),
        }
    }

    pub fn new(
        arrival: Instant,
        hash: BlockHash,
        status: InactiveCacheStatus,
        initial_rep_a: Account,
        initial_timestamp_a: u64,
    ) -> Self {
        let mut voters = Vec::new();
        voters.push((
            initial_rep_a,
            initial_timestamp_a,
        ));
        Self {
            arrival,
            hash,
            status,
            voters,
        }
    }
}

impl fmt::Display for InactiveCacheInformation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "hash={}", self.hash.to_string());
        write!(f, ", arrival={:?}", self.arrival.checked_sub(UNIX_EPOCH.elapsed().unwrap()));
        write!(f, ", {}", self.status);
        write!(f, ", {}", " voters");
        write!(f, ", {:?}", self.voters)
    }
}
