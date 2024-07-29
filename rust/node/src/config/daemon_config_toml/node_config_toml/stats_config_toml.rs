use crate::{config::Miliseconds, stats::StatsConfig};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Deserialize, Serialize)]
pub struct StatsConfigToml {
    pub max_samples: Option<usize>,
    pub log_samples_interval: Option<Miliseconds>,
    pub log_counters_interval: Option<Miliseconds>,
    pub log_rotation_count: Option<usize>,
    pub log_headers: Option<bool>,
    pub log_counters_filename: Option<String>,
    pub log_samples_filename: Option<String>,
}

impl Default for StatsConfigToml {
    fn default() -> Self {
        let config = StatsConfig::default();
        Self {
            max_samples: Some(config.max_samples),
            log_samples_interval: Some(Miliseconds(config.log_samples_interval.as_millis())),
            log_counters_interval: Some(Miliseconds(config.log_counters_interval.as_millis())),
            log_rotation_count: Some(config.log_rotation_count),
            log_headers: Some(config.log_headers),
            log_counters_filename: Some(config.log_counters_filename),
            log_samples_filename: Some(config.log_samples_filename),
        }
    }
}

impl From<&StatsConfigToml> for StatsConfig {
    fn from(toml: &StatsConfigToml) -> Self {
        let mut config = StatsConfig::default();

        if let Some(log_counters_filename) = &toml.log_counters_filename {
            config.log_counters_filename = log_counters_filename.clone();
        }
        if let Some(log_counters_interval) = &toml.log_counters_interval {
            config.log_counters_interval = Duration::from_millis(log_counters_interval.0 as u64);
        }
        if let Some(log_headers) = toml.log_headers {
            config.log_headers = log_headers;
        }
        if let Some(log_rotation_count) = toml.log_rotation_count {
            config.log_rotation_count = log_rotation_count;
        }
        if let Some(max_samples) = toml.max_samples {
            config.max_samples = max_samples;
        }
        if let Some(log_samples_filename) = &toml.log_samples_filename {
            config.log_samples_filename = log_samples_filename.clone();
        }
        if let Some(log_samples_interval) = &toml.log_samples_interval {
            config.log_samples_interval = Duration::from_millis(log_samples_interval.0 as u64);
        }
        config
    }
}

impl From<StatsConfig> for StatsConfigToml {
    fn from(config: StatsConfig) -> Self {
        Self {
            max_samples: Some(config.max_samples),
            log_samples_interval: Some(Miliseconds(config.log_samples_interval.as_millis())),
            log_counters_interval: Some(Miliseconds(config.log_counters_interval.as_millis())),
            log_rotation_count: Some(config.log_rotation_count),
            log_headers: Some(config.log_headers),
            log_counters_filename: Some(config.log_counters_filename),
            log_samples_filename: Some(config.log_samples_filename),
        }
    }
}
