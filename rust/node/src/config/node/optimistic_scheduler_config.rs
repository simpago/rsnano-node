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

#[derive(Clone, Deserialize, Serialize)]
pub struct OptimisticSchedulerConfigToml {
    pub enabled: bool,
    pub gap_threshold: u64,
    pub max_size: usize,
}

impl From<OptimisticSchedulerConfig> for OptimisticSchedulerConfigToml {
    fn from(config: OptimisticSchedulerConfig) -> Self {
        Self {
            enabled: config.enabled,
            gap_threshold: config.gap_threshold,
            max_size: config.max_size,
        }
    }
}
