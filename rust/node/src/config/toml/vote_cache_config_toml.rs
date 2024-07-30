use crate::{config::Miliseconds, consensus::VoteCacheConfig};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Deserialize, Serialize)]
pub struct VoteCacheConfigToml {
    pub max_size: Option<usize>,
    pub max_voters: Option<usize>,
    pub age_cutoff: Option<Miliseconds>,
}

impl Default for VoteCacheConfigToml {
    fn default() -> Self {
        let config = VoteCacheConfig::default();
        Self {
            max_size: Some(config.max_size),
            max_voters: Some(config.max_voters),
            age_cutoff: Some(Miliseconds(config.age_cutoff.as_millis())),
        }
    }
}

impl From<&VoteCacheConfigToml> for VoteCacheConfig {
    fn from(toml: &VoteCacheConfigToml) -> Self {
        let mut config = VoteCacheConfig::default();

        if let Some(max_size) = toml.max_size {
            config.max_size = max_size;
        }
        if let Some(max_voters) = toml.max_voters {
            config.max_voters = max_voters;
        }
        if let Some(age_cutoff) = &toml.age_cutoff {
            config.age_cutoff = Duration::from_millis(age_cutoff.0 as u64);
        }
        config
    }
}

impl From<&VoteCacheConfig> for VoteCacheConfigToml {
    fn from(config: &VoteCacheConfig) -> Self {
        Self {
            max_size: Some(config.max_size),
            max_voters: Some(config.max_voters),
            age_cutoff: Some(Miliseconds(config.age_cutoff.as_millis())),
        }
    }
}
