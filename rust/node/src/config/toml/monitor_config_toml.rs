use crate::monitor::MonitorConfig;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Deserialize, Serialize)]
pub struct MonitorConfigToml {
    pub enabled: Option<bool>,
    pub interval: Option<u64>,
}

impl Default for MonitorConfigToml {
    fn default() -> Self {
        let config = MonitorConfig::default();
        Self {
            enabled: Some(config.enabled),
            interval: Some(config.interval.as_secs()),
        }
    }
}

impl From<&MonitorConfigToml> for MonitorConfig {
    fn from(toml: &MonitorConfigToml) -> Self {
        let mut config = MonitorConfig::default();

        if let Some(enabled) = toml.enabled {
            config.enabled = enabled;
        }
        if let Some(interval) = &toml.interval {
            config.interval = Duration::from_secs(*interval);
        }
        config
    }
}

impl From<&MonitorConfig> for MonitorConfigToml {
    fn from(config: &MonitorConfig) -> Self {
        Self {
            enabled: Some(config.enabled),
            interval: Some(config.interval.as_secs()),
        }
    }
}
