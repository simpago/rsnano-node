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
        (&config).into()
    }
}

impl From<&OptimisticSchedulerToml> for OptimisticSchedulerConfig {
    fn from(toml: &OptimisticSchedulerToml) -> Self {
        let mut config = OptimisticSchedulerConfig::new();

        if let Some(enabled) = toml.enable {
            config.enabled = enabled;
        }
        if let Some(gap_threshold) = toml.gap_threshold {
            config.gap_threshold = gap_threshold;
        }
        if let Some(max_size) = toml.max_size {
            config.max_size = max_size;
        }
        config
    }
}

impl From<&OptimisticSchedulerConfig> for OptimisticSchedulerToml {
    fn from(config: &OptimisticSchedulerConfig) -> Self {
        Self {
            enable: Some(config.enabled),
            gap_threshold: Some(config.gap_threshold),
            max_size: Some(config.max_size),
        }
    }
}
