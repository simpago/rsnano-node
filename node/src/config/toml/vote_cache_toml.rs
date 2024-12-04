use crate::consensus::VoteCacheConfig;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Deserialize, Serialize)]
pub struct VoteCacheToml {
    pub age_cutoff: Option<u64>,
    pub max_size: Option<usize>,
    pub max_voters: Option<usize>,
}

impl Default for VoteCacheToml {
    fn default() -> Self {
        let config = VoteCacheConfig::default();
        (&config).into()
    }
}

impl From<&VoteCacheToml> for VoteCacheConfig {
    fn from(toml: &VoteCacheToml) -> Self {
        let mut config = VoteCacheConfig::default();

        if let Some(max_size) = toml.max_size {
            config.max_size = max_size;
        }
        if let Some(max_voters) = toml.max_voters {
            config.max_voters = max_voters;
        }
        if let Some(age_cutoff) = &toml.age_cutoff {
            config.age_cutoff = Duration::from_secs(*age_cutoff);
        }
        config
    }
}

impl From<&VoteCacheConfig> for VoteCacheToml {
    fn from(config: &VoteCacheConfig) -> Self {
        Self {
            max_size: Some(config.max_size),
            max_voters: Some(config.max_voters),
            age_cutoff: Some(config.age_cutoff.as_secs() as u64),
        }
    }
}
