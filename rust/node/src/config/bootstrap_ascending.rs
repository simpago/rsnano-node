use crate::bootstrap::{AccountSetsConfig, BootstrapAscendingConfig};
use rsnano_core::utils::TomlWriter;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Deserialize, Serialize)]
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

impl BootstrapAscendingToml {
    pub fn serialize_toml(&self, toml: &mut dyn TomlWriter) -> anyhow::Result<()> {
        toml.put_usize ("requests_limit", self.requests_limit, "Request limit to ascending bootstrap after which requests will be dropped.\nNote: changing to unlimited (0) is not recommended.\ntype:uint64")?;
        toml.put_usize ("database_requests_limit", self.database_requests_limit, "Request limit for accounts from database after which requests will be dropped.\nNote: changing to unlimited (0) is not recommended as this operation competes for resources on querying the database.\ntype:uint64")?;
        toml.put_usize(
            "pull_count",
            self.pull_count,
            "Number of requested blocks for ascending bootstrap request.\ntype:uint64",
        )?;
        toml.put_u64 ("timeout", self.timeout.as_millis() as u64, "Timeout in milliseconds for incoming ascending bootstrap messages to be processed.\ntype:milliseconds")?;
        toml.put_usize(
            "throttle_coefficient",
            self.throttle_coefficient,
            "Scales the number of samples to track for bootstrap throttling.\ntype:uint64",
        )?;
        toml.put_u64(
            "throttle_wait",
            self.throttle_wait.as_millis() as u64,
            "Length of time to wait between requests when throttled.\ntype:milliseconds",
        )?;
        toml.put_usize(
            "block_wait_count",
            self.block_wait_count,
            "Ascending bootstrap will wait while block processor has more than this many blocks queued.\ntype:uint64",
        )?;
        toml.put_child("account_sets", &mut |child| {
            self.account_sets.serialize_toml(child)
        })
    }
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

#[derive(Clone, Deserialize, Serialize)]
pub struct AccountSetsToml {
    pub consideration_count: usize,
    pub priorities_max: usize,
    pub blocking_max: usize,
    pub cooldown: Duration,
}

impl AccountSetsToml {
    pub(crate) fn serialize_toml(&self, toml: &mut dyn TomlWriter) -> anyhow::Result<()> {
        toml.put_usize ("consideration_count", self.consideration_count, "Limit the number of account candidates to consider and also the number of iterations.\ntype:uint64")?;
        toml.put_usize(
            "priorities_max",
            self.priorities_max,
            "Cutoff size limit for the priority list.\ntype:uint64",
        )?;
        toml.put_usize(
            "blocking_max",
            self.blocking_max,
            "Cutoff size limit for the blocked accounts from the priority list.\ntype:uint64",
        )?;
        toml.put_u64(
            "cooldown",
            self.cooldown.as_millis() as u64,
            "Waiting time for an account to become available.\ntype:milliseconds",
        )
    }
}

impl Default for AccountSetsToml {
    fn default() -> Self {
        (&AccountSetsConfig::default()).into()
    }
}
