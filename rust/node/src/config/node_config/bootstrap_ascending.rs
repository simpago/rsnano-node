use crate::bootstrap::AccountSetsConfig;
use rsnano_messages::BlocksAckPayload;
use std::time::Duration;

#[derive(Clone)]
pub struct BootstrapAscendingConfig {
    /// Maximum number of un-responded requests per channel
    pub requests_limit: usize,
    pub database_requests_limit: usize,
    pub pull_count: usize,
    pub timeout: Duration,
    pub throttle_coefficient: usize,
    pub throttle_wait: Duration,
    pub account_sets: AccountSetsConfig,
    pub block_wait_count: usize,
    /** Minimum accepted protocol version used when bootstrapping */
    pub min_protocol_version: u8,
}

impl Default for BootstrapAscendingConfig {
    fn default() -> Self {
        Self {
            requests_limit: 64,
            database_requests_limit: 1024,
            pull_count: BlocksAckPayload::MAX_BLOCKS,
            timeout: Duration::from_secs(3),
            throttle_coefficient: 16,
            throttle_wait: Duration::from_millis(100),
            account_sets: Default::default(),
            block_wait_count: 1000,
            min_protocol_version: 0x14,
        }
    }
}
