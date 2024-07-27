use crate::config::{Miliseconds, StatsConfig};
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
pub struct StatsConfigToml {
    pub max_samples: usize,
    pub log_samples_interval: Miliseconds,
    pub log_counters_interval: Miliseconds,
    pub log_rotation_count: usize,
    pub log_headers: bool,
    pub log_counters_filename: String,
    pub log_samples_filename: String,
}

impl From<StatsConfig> for StatsConfigToml {
    fn from(config: StatsConfig) -> Self {
        Self {
            max_samples: config.max_samples,
            log_samples_interval: Miliseconds(config.log_samples_interval.as_millis()),
            log_counters_interval: Miliseconds(config.log_counters_interval.as_millis()),
            log_rotation_count: config.log_rotation_count,
            log_headers: config.log_headers,
            log_counters_filename: config.log_counters_filename,
            log_samples_filename: config.log_samples_filename,
        }
    }
}
