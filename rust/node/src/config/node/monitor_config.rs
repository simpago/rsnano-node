use rsnano_core::utils::TomlWriter;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone)]
pub struct MonitorConfig {
    pub enabled: bool,
    pub interval: Duration,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval: Duration::from_secs(60),
        }
    }
}

impl MonitorConfig {
    pub fn serialize_toml(&self, toml: &mut dyn TomlWriter) -> anyhow::Result<()> {
        toml.put_bool(
            "enable",
            self.enabled,
            "Enable or disable periodic node status logging\ntype:bool",
        )?;

        toml.put_u64(
            "interval",
            self.interval.as_secs(),
            "Interval between status logs\ntype:seconds",
        )
    }
}

#[derive(Deserialize, Serialize)]
pub struct MonitorConfigToml {
    pub enabled: Option<bool>,
    pub interval: Option<u64>,
}

impl From<&MonitorConfig> for MonitorConfigToml {
    fn from(config: &MonitorConfig) -> Self {
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
