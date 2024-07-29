use crate::monitor::MonitorConfig;
use rsnano_core::utils::TomlWriter;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Deserialize, Serialize)]
pub struct MonitorConfigToml {
    pub enabled: Option<bool>,
    pub interval: Option<u64>,
}

impl MonitorConfigToml {
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
