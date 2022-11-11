use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::core::{Account, Amount, BlockEnum, SignatureVerification, UncheckedInfo};
use crate::utils::{MemoryStream, Serialize};

/**
 * Tag for the type of the election status
 */
#[repr(u8)]
#[derive(PartialEq, Eq, Debug, Clone, Copy, FromPrimitive)]
pub enum ElectionStatusType {
    Ongoing = 0,
    ActiveConfirmedQuorum = 1,
    ActiveConfirmationHeight = 2,
    InactiveConfirmationHeight = 3,
    Stopped = 5
}

/// Information on the status of an election
#[derive(Clone)]
pub struct ElectionStatus {
    pub winner: Option<Arc<RwLock<BlockEnum>>>,
    pub tally: Amount,
    pub final_tally: Amount,
    pub confirmation_request_count: u32,
    pub block_count: u32,
    pub voter_count: u32,
    pub election_end: i64,
    pub election_duration: i64,
    pub election_status_type: ElectionStatusType,
}

impl ElectionStatus {
    pub(crate) fn new(
        winner: Arc<RwLock<BlockEnum>>,
        tally: &Amount,
        final_tally: &Amount,
        confirmation_request_count: u32,
        block_count: u32,
        voter_count: u32,
        election_end: i64,
        election_duration: i64,
        election_status_type: ElectionStatusType,
    ) -> Self {
        Self {
            winner: Some(winner),
            tally: *tally,
            final_tally: *final_tally,
            confirmation_request_count,
            block_count,
            voter_count,
            election_end,
            election_duration,
            election_status_type,
        }
    }

    pub(crate) fn null() -> Self {
        Self {
            winner: None,
            tally: Amount::zero(),
            final_tally: Amount::zero(),
            confirmation_request_count: 0,
            block_count: 0,
            voter_count: 0,
            election_end: 0,
            election_duration: 0,
            election_status_type: ElectionStatusType::Stopped,
        }
    }
}
