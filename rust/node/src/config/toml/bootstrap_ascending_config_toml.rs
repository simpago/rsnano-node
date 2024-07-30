use super::Miliseconds;
use crate::bootstrap::{AccountSetsConfig, BootstrapAscendingConfig};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Deserialize, Serialize)]
pub struct BootstrapAscendingConfigToml {
    pub requests_limit: Option<usize>,
    pub database_requests_limit: Option<usize>,
    pub pull_count: Option<usize>,
    pub timeout: Option<Miliseconds>,
    pub throttle_coefficient: Option<usize>,
    pub throttle_wait: Option<Miliseconds>,
    pub account_sets: Option<AccountSetsConfigToml>,
    pub block_wait_count: Option<usize>,
}

impl Default for BootstrapAscendingConfigToml {
    fn default() -> Self {
        let config = BootstrapAscendingConfig::default();
        Self {
            requests_limit: Some(config.requests_limit),
            database_requests_limit: Some(config.database_requests_limit),
            pull_count: Some(config.pull_count),
            timeout: Some(Miliseconds(config.timeout.as_millis())),
            throttle_coefficient: Some(config.throttle_coefficient),
            throttle_wait: Some(Miliseconds(config.throttle_wait.as_millis())),
            account_sets: Some(AccountSetsConfigToml::default()),
            block_wait_count: Some(config.block_wait_count),
        }
    }
}

impl From<&BootstrapAscendingConfig> for BootstrapAscendingConfigToml {
    fn from(config: &BootstrapAscendingConfig) -> Self {
        Self {
            requests_limit: Some(config.requests_limit),
            database_requests_limit: Some(config.database_requests_limit),
            pull_count: Some(config.pull_count),
            timeout: Some(Miliseconds(config.timeout.as_millis())),
            throttle_coefficient: Some(config.throttle_coefficient),
            throttle_wait: Some(Miliseconds(config.throttle_wait.as_millis())),
            account_sets: Some((&config.account_sets).into()),
            block_wait_count: Some(config.block_wait_count),
        }
    }
}

impl From<&BootstrapAscendingConfigToml> for BootstrapAscendingConfig {
    fn from(toml: &BootstrapAscendingConfigToml) -> Self {
        let mut config = BootstrapAscendingConfig::default();

        if let Some(account_sets) = &toml.account_sets {
            config.account_sets = account_sets.into();
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

#[derive(Clone, Deserialize, Serialize)]
pub struct AccountSetsConfigToml {
    pub consideration_count: Option<usize>,
    pub priorities_max: Option<usize>,
    pub blocking_max: Option<usize>,
    pub cooldown: Option<Miliseconds>,
}

impl Default for AccountSetsConfigToml {
    fn default() -> Self {
        let config = AccountSetsConfig::default();
        Self {
            consideration_count: Some(config.consideration_count),
            priorities_max: Some(config.priorities_max),
            blocking_max: Some(config.blocking_max),
            cooldown: Some(Miliseconds(config.cooldown.as_millis())),
        }
    }
}

impl From<&AccountSetsConfigToml> for AccountSetsConfig {
    fn from(toml: &AccountSetsConfigToml) -> Self {
        let mut config = AccountSetsConfig::default();

        if let Some(blocking_max) = toml.blocking_max {
            config.blocking_max = blocking_max;
        }
        if let Some(consideration_count) = toml.consideration_count {
            config.consideration_count = consideration_count;
        }
        if let Some(priorities_max) = toml.priorities_max {
            config.priorities_max = priorities_max;
        }
        if let Some(cooldown) = &toml.cooldown {
            config.cooldown = Duration::from_millis(cooldown.0 as u64);
        }
        config
    }
}

impl From<&AccountSetsConfig> for AccountSetsConfigToml {
    fn from(value: &AccountSetsConfig) -> Self {
        Self {
            consideration_count: Some(value.consideration_count),
            priorities_max: Some(value.priorities_max),
            blocking_max: Some(value.blocking_max),
            cooldown: Some(Miliseconds(value.cooldown.as_millis())),
        }
    }
}
