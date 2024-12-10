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
        Self {
            enable: Some(true),
            hinting_threshold: Some(config.hinting_threshold_percent),
            block_cooldown: Some(config.block_cooldown.as_millis() as u64),
            check_interval: Some(config.check_interval.as_millis() as u64),
            vacancy_threshold: Some(config.vacancy_threshold_percent),
        }
    }
}

impl HintedSchedulerConfig {
    pub fn merge_toml(&mut self, toml: &HintedSchedulerToml) {
        if let Some(block_cooldown) = toml.block_cooldown {
            self.block_cooldown = Duration::from_millis(block_cooldown);
        }
        if let Some(check_interval) = toml.check_interval {
            self.check_interval = Duration::from_millis(check_interval);
        }
        if let Some(hinting_threshold) = toml.hinting_threshold {
            self.hinting_threshold_percent = hinting_threshold;
        }
        if let Some(vacancy_threshold) = toml.vacancy_threshold {
            self.vacancy_threshold_percent = vacancy_threshold;
        }
    }
}
