use crate::stats::StatsConfig;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Deserialize, Serialize)]
pub struct LogToml {
    pub filename_counters: Option<String>,
    pub filename_samples: Option<String>,
    pub headers: Option<bool>,
    pub interval_counters: Option<u64>,
    pub interval_samples: Option<u64>,
    pub rotation_count: Option<usize>,
}

impl Default for LogToml {
    fn default() -> Self {
        let config = StatsConfig::default();
        (&config).into()
    }
}

#[derive(Deserialize, Serialize)]
pub struct StatsToml {
    pub max_samples: Option<usize>,
    pub log: Option<LogToml>,
}

impl Default for StatsToml {
    fn default() -> Self {
        let config = StatsConfig::default();
        (&config).into()
    }
}

impl From<&StatsToml> for StatsConfig {
    fn from(toml: &StatsToml) -> Self {
        let mut config = StatsConfig::default();

        if let Some(max_samples) = toml.max_samples {
            config.max_samples = max_samples;
        }
        if let Some(log) = &toml.log {
            if let Some(log_counters_filename) = &log.filename_counters {
                config.log_counters_filename = log_counters_filename.clone();
            }
            if let Some(log_counters_interval) = &log.interval_counters {
                config.log_counters_interval = Duration::from_millis(*log_counters_interval);
            }
            if let Some(log_headers) = log.headers {
                config.log_headers = log_headers;
            }
            if let Some(log_rotation_count) = log.rotation_count {
                config.log_rotation_count = log_rotation_count;
            }
            if let Some(log_samples_filename) = &log.filename_samples {
                config.log_samples_filename = log_samples_filename.clone();
            }
            if let Some(log_samples_interval) = &log.interval_samples {
                config.log_samples_interval = Duration::from_millis(*log_samples_interval);
            }
        }
        config
    }
}

impl From<&StatsConfig> for StatsToml {
    fn from(config: &StatsConfig) -> Self {
        Self {
            max_samples: Some(config.max_samples),
            log: Some(config.into()),
        }
    }
}

impl From<&StatsConfig> for LogToml {
    fn from(config: &StatsConfig) -> Self {
        Self {
            interval_samples: Some(config.log_samples_interval.as_millis() as u64),
            interval_counters: Some(config.log_counters_interval.as_millis() as u64),
            rotation_count: Some(config.log_rotation_count),
            headers: Some(config.log_headers),
            filename_counters: Some(config.log_counters_filename.clone()),
            filename_samples: Some(config.log_samples_filename.clone()),
        }
    }
}
