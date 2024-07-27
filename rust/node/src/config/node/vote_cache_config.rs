use rsnano_core::utils::TomlWriter;
use std::time::Duration;

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
