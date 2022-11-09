use std::sync::{
    atomic::{AtomicBool, AtomicU64},
    Arc,
};

use super::RepWeights;

pub struct LedgerCache {
    pub rep_weights: Arc<RepWeights>,
    pub cemented_count: AtomicU64,
    pub block_count: AtomicU64,
    pub pruned_count: AtomicU64,
    pub account_count: AtomicU64,
    pub final_votes_confirmation_canary: AtomicBool,
}

impl LedgerCache {
    pub fn new() -> Self {
        let rep_weights = Arc::new(RepWeights::new());
        Self {
            rep_weights,
            cemented_count: AtomicU64::new(0),
            block_count: AtomicU64::new(0),
            pruned_count: AtomicU64::new(0),
            account_count: AtomicU64::new(0),
            final_votes_confirmation_canary: AtomicBool::new(false),
        }
    }
}
