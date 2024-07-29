use rsnano_core::utils::TomlWriter;
use serde::{Deserialize, Serialize};

use crate::consensus::OptimisticSchedulerConfig;

#[derive(Deserialize, Serialize)]
pub struct OptimisticSchedulerConfigToml {
    pub enabled: Option<bool>,
    pub gap_threshold: Option<u64>,
    pub max_size: Option<usize>,
}

impl OptimisticSchedulerConfigToml {
    pub(crate) fn serialize_toml(&self, toml: &mut dyn TomlWriter) -> anyhow::Result<()> {
        toml.put_bool(
            "enable",
            self.enabled,
            "Enable or disable optimistic elections\ntype:bool",
        )?;
        toml.put_u64 ("gap_threshold", self.gap_threshold, "Minimum difference between confirmation frontier and account frontier to become a candidate for optimistic confirmation\ntype:uint64")?;
        toml.put_usize(
            "max_size",
            self.max_size,
            "Maximum number of candidates stored in memory\ntype:uint64",
        )
    }
}

impl Default for OptimisticSchedulerConfigToml {
    fn default() -> Self {
        let config = OptimisticSchedulerConfig::new();
        Self {
            enabled: Some(config.enabled),
            gap_threshold: Some(config.gap_threshold),
            max_size: Some(config.max_size),
        }
    }
}

impl From<&OptimisticSchedulerConfigToml> for OptimisticSchedulerConfig {
    fn from(toml: &OptimisticSchedulerConfigToml) -> Self {
        let mut config = OptimisticSchedulerConfig::new();

        if let Some(enabled) = toml.enabled {
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
