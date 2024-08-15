use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::consensus::HintedSchedulerConfig;

#[derive(Deserialize, Serialize)]
pub struct HintedSchedulerToml {
    pub enable: Option<bool>,
    pub hinting_threshold: Option<u32>,
    pub check_interval: Option<u64>,
    pub block_cooldown: Option<u64>,
    pub vacancy_threshold: Option<u32>,
}

impl Default for HintedSchedulerToml {
    fn default() -> Self {
        let config = HintedSchedulerConfig::default();
        (&config).into()
    }
}

impl From<&HintedSchedulerToml> for HintedSchedulerConfig {
    fn from(toml: &HintedSchedulerToml) -> Self {
        let mut config = HintedSchedulerConfig::default();

        if let Some(enabled) = toml.enable {
            config.enabled = enabled;
        }
        if let Some(block_cooldown) = toml.block_cooldown {
            config.block_cooldown = Duration::from_millis(block_cooldown);
        }
        if let Some(check_interval) = toml.check_interval {
            config.check_interval = Duration::from_millis(check_interval);
        }
        if let Some(hinting_threshold) = toml.hinting_threshold {
            config.hinting_threshold_percent = hinting_threshold;
        }
        if let Some(vacancy_threshold) = toml.vacancy_threshold {
            config.vacancy_threshold_percent = vacancy_threshold;
        }
        config
    }
}

impl From<&HintedSchedulerConfig> for HintedSchedulerToml {
    fn from(config: &HintedSchedulerConfig) -> Self {
        Self {
            enable: Some(config.enabled),
            hinting_threshold: Some(config.hinting_threshold_percent),
            check_interval: Some(config.check_interval.as_millis() as u64),
            block_cooldown: Some(config.block_cooldown.as_millis() as u64),
            vacancy_threshold: Some(config.vacancy_threshold_percent),
        }
    }
}
