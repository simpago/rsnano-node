use crate::config::TomlConfigOverride;
use rsnano_core::utils::TomlWriter;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct OptimisticSchedulerConfig {
    pub enabled: bool,

    /// Minimum difference between confirmation frontier and account frontier to become a candidate for optimistic confirmation
    pub gap_threshold: u64,

    /// Maximum number of candidates stored in memory
    pub max_size: usize,
}

impl OptimisticSchedulerConfig {
    pub fn new() -> Self {
        Self {
            enabled: true,
            gap_threshold: 32,
            max_size: 1024 * 64,
        }
    }

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

#[derive(Deserialize, Serialize)]
pub struct OptimisticSchedulerConfigToml {
    pub enabled: Option<bool>,
    pub gap_threshold: Option<u64>,
    pub max_size: Option<usize>,
}

impl From<OptimisticSchedulerConfig> for OptimisticSchedulerConfigToml {
    fn from(config: OptimisticSchedulerConfig) -> Self {
        Self {
            enabled: Some(config.enabled),
            gap_threshold: Some(config.gap_threshold),
            max_size: Some(config.max_size),
        }
    }
}

impl<'de> TomlConfigOverride<'de, OptimisticSchedulerConfigToml> for OptimisticSchedulerConfig {
    fn toml_config_override(&mut self, toml: &'de OptimisticSchedulerConfigToml) {
        if let Some(enabled) = toml.enabled {
            self.enabled = enabled;
        }
        if let Some(gap_threshold) = toml.gap_threshold {
            self.gap_threshold = gap_threshold;
        }
        if let Some(max_size) = toml.max_size {
            self.max_size = max_size;
        }
    }
}
