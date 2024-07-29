use crate::consensus::PriorityBucketConfig;
use rsnano_core::utils::TomlWriter;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct PriorityBucketConfigToml {
    pub max_blocks: Option<usize>,
    pub reserved_elections: Option<usize>,
    pub max_elections: Option<usize>,
}

impl PriorityBucketConfigToml {
    pub(crate) fn serialize_toml(&self, toml: &mut dyn TomlWriter) -> anyhow::Result<()> {
        toml.put_usize(
            "max_blocks",
            self.max_blocks,
            "Maximum number of blocks to sort by priority per bucket. \nType: uint64",
        )?;
        toml.put_usize ("reserved_elections", self.reserved_elections, "Number of guaranteed slots per bucket available for election activation. \nType: uint64")?;
        toml.put_usize ("max_elections", self.max_elections, "Maximum number of slots per bucket available for election activation if the active election count is below the configured limit. \nType: uint64")
    }
}

impl Default for PriorityBucketConfigToml {
    fn default() -> Self {
        let config = PriorityBucketConfig::default();
        Self {
            max_blocks: Some(config.max_blocks),
            reserved_elections: Some(config.max_elections),
            max_elections: Some(config.reserved_elections),
        }
    }
}

impl From<&PriorityBucketConfigToml> for PriorityBucketConfig {
    fn from(toml: &PriorityBucketConfigToml) -> Self {
        let mut config = PriorityBucketConfig::default();

        if let Some(max_blocks) = toml.max_blocks {
            config.max_blocks = max_blocks;
        }
        if let Some(max_elections) = toml.max_elections {
            config.max_elections = max_elections;
        }
        if let Some(reserved_elections) = toml.reserved_elections {
            config.reserved_elections = reserved_elections;
        }
        config
    }
}
