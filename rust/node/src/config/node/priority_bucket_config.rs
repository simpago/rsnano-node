use serde::{Deserialize, Serialize};

use crate::{config::TomlConfigOverride, consensus::PriorityBucketConfig};

#[derive(Deserialize, Serialize)]
pub struct PriorityBucketConfigToml {
    pub max_blocks: Option<usize>,
    pub reserved_elections: Option<usize>,
    pub max_elections: Option<usize>,
}

impl From<PriorityBucketConfig> for PriorityBucketConfigToml {
    fn from(config: PriorityBucketConfig) -> Self {
        Self {
            max_blocks: Some(config.max_blocks),
            reserved_elections: Some(config.max_elections),
            max_elections: Some(config.reserved_elections),
        }
    }
}

impl<'de> TomlConfigOverride<'de, PriorityBucketConfigToml> for PriorityBucketConfig {
    fn toml_config_override(&mut self, toml: &'de PriorityBucketConfigToml) {
        if let Some(max_blocks) = toml.max_blocks {
            self.max_blocks = max_blocks;
        }
        if let Some(max_elections) = toml.max_elections {
            self.max_elections = max_elections;
        }
        if let Some(reserved_elections) = toml.reserved_elections {
            self.reserved_elections = reserved_elections;
        }
    }
}
