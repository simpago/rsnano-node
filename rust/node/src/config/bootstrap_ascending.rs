use crate::bootstrap::{AccountSetsConfig, BootstrapAscendingConfig};
use std::time::Duration;

#[derive(Clone)]
pub struct BootstrapAscendingToml {
    /// Maximum number of un-responded requests per channel
    pub requests_limit: usize,
    pub database_requests_limit: usize,
    pub pull_count: usize,
    pub timeout: Duration,
    pub throttle_coefficient: usize,
    pub throttle_wait: Duration,
    pub account_sets: AccountSetsToml,
    pub block_wait_count: usize,
}

impl Default for BootstrapAscendingToml {
    fn default() -> Self {
        let config = BootstrapAscendingConfig::default();
        Self {
            requests_limit: config.requests_limit,
            database_requests_limit: config.database_requests_limit,
            pull_count: config.pull_count,
            timeout: config.timeout,
            throttle_coefficient: config.throttle_coefficient,
            throttle_wait: config.throttle_wait,
            account_sets: (&config.account_sets).into(),
            block_wait_count: config.block_wait_count,
        }
    }
}

#[derive(Clone)]
pub struct AccountSetsToml {
    pub consideration_count: usize,
    pub priorities_max: usize,
    pub blocking_max: usize,
    pub cooldown: Duration,
}

impl Default for AccountSetsToml {
    fn default() -> Self {
        (&AccountSetsConfig::default()).into()
    }
}
