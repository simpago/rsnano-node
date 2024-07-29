use crate::{bootstrap::BootstrapAscendingConfig, config::Miliseconds};
use rsnano_core::utils::TomlWriter;
use std::time::Duration;

#[derive(Clone)]
pub struct BootstrapAscendingConfigToml {
    /// Maximum number of un-responded requests per channel
    pub requests_limit: Option<usize>,
    pub database_requests_limit: Option<usize>,
    pub pull_count: Option<usize>,
    pub timeout: Option<Miliseconds>,
    pub throttle_coefficient: Option<usize>,
    pub throttle_wait: Option<Miliseconds>,
    pub account_sets: Option<AccountSetsToml>,
    pub block_wait_count: Option<usize>,
}

impl BootstrapAscendingConfigToml {
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

impl Default for BootstrapAscendingConfigToml {
    fn default() -> Self {
        let config = BootstrapAscendingConfig::default();
        Self {
            requests_limit: Some(config.requests_limit),
            database_requests_limit: Some(config.database_requests_limit),
            pull_count: Some(config.pull_count),
            timeout: Some(config.timeout),
            throttle_coefficient: Some(config.throttle_coefficient),
            throttle_wait: Some(config.throttle_wait),
            account_sets: Some(config.account_sets),
            block_wait_count: Some(config.block_wait_count),
        }
    }
}

impl From<&BootstrapAscendingConfigToml> for BootstrapAscendingConfig {
    fn from(toml: &BootstrapAscendingConfigToml) -> Self {
        let mut config = BootstrapAscendingConfig::default();

        if let Some(account_sets) = &toml.account_sets {
            if let Some(blocking_max) = account_sets.blocking_max {
                config.account_sets.blocking_max = blocking_max;
            }
            if let Some(consideration_count) = account_sets.consideration_count {
                config.account_sets.consideration_count = consideration_count;
            }
            if let Some(priorities_max) = account_sets.priorities_max {
                config.account_sets.priorities_max = priorities_max;
            }
            if let Some(cooldown) = &account_sets.cooldown {
                config.account_sets.cooldown = Duration::from_millis(cooldown.0 as u64);
            }
        }
        if let Some(block_wait_count) = toml.block_wait_count {
            config.block_wait_count = block_wait_count;
        }
        if let Some(database_requests_limit) = toml.database_requests_limit {
            config.database_requests_limit = database_requests_limit;
        }
        if let Some(pull_count) = toml.pull_count {
            config.pull_count = pull_count;
        }
        if let Some(requests_limit) = toml.requests_limit {
            config.requests_limit = requests_limit;
        }
        if let Some(timeout) = &toml.timeout {
            config.timeout = Duration::from_millis(timeout.0 as u64);
        }
        if let Some(throttle_wait) = &toml.throttle_wait {
            config.throttle_wait = Duration::from_millis(throttle_wait.0 as u64);
        }
        if let Some(throttle_coefficient) = toml.throttle_coefficient {
            config.throttle_coefficient = throttle_coefficient;
        }
        config
    }
}

#[derive(Clone)]
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
