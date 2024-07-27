use rsnano_core::utils::TomlWriter;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::config::{Miliseconds, TomlConfigOverride};

#[derive(Clone)]
pub struct VoteCacheConfig {
    pub max_size: usize,
    pub max_voters: usize,
    pub age_cutoff: Duration,
}

impl VoteCacheConfig {
    pub fn serialize_toml(&self, toml: &mut dyn TomlWriter) -> anyhow::Result<()> {
        toml.put_usize(
            "max_size",
            self.max_size,
            "Maximum number of blocks to cache votes for. \ntype:uint64",
        )?;

        toml.put_usize(
            "max_voters",
            self.max_voters,
            "Maximum number of voters to cache per block. \ntype:uint64",
        )?;

        toml.put_u64(
            "age_cutoff",
            self.age_cutoff.as_secs(),
            "Maximum age of votes to keep in cache. \ntype:seconds",
        )
    }
}

impl Default for VoteCacheConfig {
    fn default() -> Self {
        Self {
            max_size: 1024 * 64,
            max_voters: 64,
            age_cutoff: Duration::from_secs(15 * 60),
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct VoteCacheConfigToml {
    pub max_size: Option<usize>,
    pub max_voters: Option<usize>,
    pub age_cutoff: Option<Miliseconds>,
}

impl From<VoteCacheConfig> for VoteCacheConfigToml {
    fn from(config: VoteCacheConfig) -> Self {
        Self {
            max_size: Some(config.max_size),
            max_voters: Some(config.max_voters),
            age_cutoff: Some(Miliseconds(config.age_cutoff.as_millis())),
        }
    }
}

impl<'de> TomlConfigOverride<'de, VoteCacheConfigToml> for VoteCacheConfig {
    fn toml_config_override(&mut self, toml: &'de VoteCacheConfigToml) {
        if let Some(max_size) = toml.max_size {
            self.max_size = max_size;
        }
        if let Some(max_voters) = toml.max_voters {
            self.max_voters = max_voters;
        }
        if let Some(age_cutoff) = &toml.age_cutoff {
            self.age_cutoff = Duration::from_millis(age_cutoff.0 as u64);
        }
    }
}
