use crate::{config::Miliseconds, stats::StatsConfig};
use anyhow::Result;
use rsnano_core::utils::TomlWriter;
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

impl StatsConfigToml {
    pub fn serialize_toml(&self, toml: &mut dyn TomlWriter) -> Result<()> {
        toml.put_usize(
            "max_samples",
            self.max_samples,
            "Maximum number ofmany samples to keep in the ring buffer.\ntype:uint64",
        )?;

        toml.put_child("log", &mut |log|{
            log.put_bool("headers", self.log_headers, "If true, write headers on each counter or samples writeout.\nThe header contains log type and the current wall time.\ntype:bool")?;
            log.put_usize("interval_counters", self.log_counters_interval.as_millis() as usize, "How often to log counters. 0 disables logging.\ntype:milliseconds")?;
            log.put_usize("interval_samples", self.log_samples_interval.as_millis() as usize, "How often to log samples. 0 disables logging.\ntype:milliseconds")?;
            log.put_usize("rotation_count", self.log_rotation_count, "Maximum number of log outputs before rotating the file.\ntype:uint64")?;
            log.put_str("filename_counters", &self.log_counters_filename, "Log file name for counters.\ntype:string")?;
            log.put_str("filename_samples", &self.log_samples_filename, "Log file name for samples.\ntype:string")?;
            Ok(())
        })?;
        Ok(())
    }
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
