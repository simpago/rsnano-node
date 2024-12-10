use crate::consensus::OptimisticSchedulerConfig;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct OptimisticSchedulerToml {
    pub enable: Option<bool>,
    pub gap_threshold: Option<u64>,
    pub max_size: Option<usize>,
}

impl Default for OptimisticSchedulerToml {
    fn default() -> Self {
        let config = OptimisticSchedulerConfig::new();
        Self {
            enable: Some(true),
            gap_threshold: Some(config.gap_threshold),
            max_size: Some(config.max_size),
        }
    }
}

impl OptimisticSchedulerConfig {
    pub fn merge_toml(&mut self, toml: &OptimisticSchedulerToml) {
        if let Some(gap_threshold) = toml.gap_threshold {
            self.gap_threshold = gap_threshold;
        }
        if let Some(max_size) = toml.max_size {
            self.max_size = max_size;
        }
    }
}
