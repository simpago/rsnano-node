use crate::consensus::PriorityBucketConfig;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct PriorityBucketConfigToml {
    pub max_blocks: Option<usize>,
    pub reserved_elections: Option<usize>,
    pub max_elections: Option<usize>,
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
