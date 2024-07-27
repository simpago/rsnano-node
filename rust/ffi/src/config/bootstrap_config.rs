use rsnano_node::{
    bootstrap::AccountSetsConfig,
    config::{AccountSetsToml, BootstrapAscendingConfig, BootstrapAscendingConfigToml},
};
use std::time::Duration;

#[repr(C)]
pub struct BootstrapAscendingConfigDto {
    pub requests_limit: usize,
    pub database_requests_limit: usize,
    pub pull_count: usize,
    pub timeout_ms: u64,
    pub throttle_coefficient: usize,
    pub throttle_wait_ms: u64,
    pub account_sets: AccountSetsConfigDto,
    pub block_wait_count: usize,
}

#[repr(C)]
pub struct AccountSetsConfigDto {
    pub consideration_count: usize,
    pub priorities_max: usize,
    pub blocking_max: usize,
    pub cooldown_ms: u64,
}

impl From<&BootstrapAscendingConfig> for BootstrapAscendingConfigDto {
    fn from(value: &BootstrapAscendingConfig) -> Self {
        Self {
            requests_limit: value.requests_limit,
            database_requests_limit: value.database_requests_limit,
            pull_count: value.pull_count,
            timeout_ms: value.timeout.as_millis() as u64,
            throttle_coefficient: value.throttle_coefficient,
            throttle_wait_ms: value.throttle_wait.as_millis() as u64,
            account_sets: (&value.account_sets).into(),
            block_wait_count: value.block_wait_count,
        }
    }
}

impl From<&BootstrapAscendingConfigDto> for BootstrapAscendingConfigToml {
    fn from(value: &BootstrapAscendingConfigDto) -> Self {
        Self {
            requests_limit: value.requests_limit,
            database_requests_limit: value.database_requests_limit,
            pull_count: value.pull_count,
            timeout: Duration::from_millis(value.timeout_ms),
            throttle_coefficient: value.throttle_coefficient,
            throttle_wait: Duration::from_millis(value.throttle_wait_ms),
            account_sets: (&value.account_sets).into(),
            block_wait_count: value.block_wait_count,
        }
    }
}

impl From<&BootstrapAscendingConfigDto> for BootstrapAscendingConfig {
    fn from(value: &BootstrapAscendingConfigDto) -> Self {
        Self {
            requests_limit: value.requests_limit,
            database_requests_limit: value.database_requests_limit,
            pull_count: value.pull_count,
            timeout: Duration::from_millis(value.timeout_ms),
            throttle_coefficient: value.throttle_coefficient,
            throttle_wait: Duration::from_millis(value.throttle_wait_ms),
            account_sets: AccountSetsConfig::default(),
            block_wait_count: value.block_wait_count,
            min_protocol_version: 0,
        }
    }
}

impl From<&AccountSetsConfig> for AccountSetsConfigDto {
    fn from(value: &AccountSetsConfig) -> Self {
        Self {
            consideration_count: value.consideration_count,
            priorities_max: value.priorities_max,
            blocking_max: value.blocking_max,
            cooldown_ms: value.cooldown.as_millis() as u64,
        }
    }
}

impl From<&AccountSetsConfigDto> for AccountSetsToml {
    fn from(value: &AccountSetsConfigDto) -> Self {
        Self {
            consideration_count: value.consideration_count,
            priorities_max: value.priorities_max,
            blocking_max: value.blocking_max,
            cooldown: Duration::from_millis(value.cooldown_ms),
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_account_sets_config_create(result: *mut AccountSetsConfigDto) {
    (*result) = (&AccountSetsConfig::default()).into()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_bootstrap_config_create(result: *mut BootstrapAscendingConfigDto) {
    (*result) = (&BootstrapAscendingConfig::default()).into()
}
