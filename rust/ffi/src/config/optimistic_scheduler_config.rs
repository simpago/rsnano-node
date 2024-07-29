use std::time::Duration;

use rsnano_node::consensus::{HintedSchedulerConfig, OptimisticSchedulerConfig};

#[repr(C)]
pub struct OptimisticSchedulerConfigDto {
    pub enabled: bool,
    pub gap_threshold: u64,
    pub max_size: usize,
}

impl From<&OptimisticSchedulerConfigDto> for OptimisticSchedulerConfig {
    fn from(value: &OptimisticSchedulerConfigDto) -> Self {
        Self {
            enabled: value.enabled,
            gap_threshold: value.gap_threshold,
            max_size: value.max_size,
        }
    }
}

impl From<&OptimisticSchedulerConfig> for OptimisticSchedulerConfigDto {
    fn from(value: &OptimisticSchedulerConfig) -> Self {
        Self {
            enabled: value.enabled,
            gap_threshold: value.gap_threshold,
            max_size: value.max_size,
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_optimistic_scheduler_config_create(
    dto: *mut OptimisticSchedulerConfigDto,
) {
    *dto = (&OptimisticSchedulerConfig::new()).into()
}

#[repr(C)]
pub struct HintedSchedulerConfigDto {
    pub enabled: bool,
    pub check_interval_ms: u32,
    pub block_cooldown_ms: u32,
    pub hinting_threshold_percent: u32,
    pub vacancy_threshold_percent: u32,
}

#[no_mangle]
pub unsafe extern "C" fn rsn_hinted_scheduler_config_create(
    is_dev_network: bool,
    dto: *mut HintedSchedulerConfigDto,
) {
    let config = if is_dev_network {
        HintedSchedulerConfig::default_for_dev_network()
    } else {
        HintedSchedulerConfig::default()
    };
    *dto = (&config).into()
}

impl From<&HintedSchedulerConfig> for HintedSchedulerConfigDto {
    fn from(value: &HintedSchedulerConfig) -> Self {
        Self {
            enabled: value.enabled,
            check_interval_ms: value.check_interval.as_millis() as u32,
            block_cooldown_ms: value.block_cooldown.as_millis() as u32,
            hinting_threshold_percent: value.hinting_theshold_percent,
            vacancy_threshold_percent: value.vacancy_threshold_percent,
        }
    }
}

impl From<&HintedSchedulerConfigDto> for HintedSchedulerConfig {
    fn from(value: &HintedSchedulerConfigDto) -> Self {
        Self {
            enabled: value.enabled,
            check_interval: Duration::from_millis(value.check_interval_ms as u64),
            block_cooldown: Duration::from_millis(value.block_cooldown_ms as u64),
            hinting_theshold_percent: value.hinting_threshold_percent,
            vacancy_threshold_percent: value.vacancy_threshold_percent,
        }
    }
}
