use rsnano_store_lmdb::{LmdbConfig, SyncStrategy};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct LmdbConfigToml {
    pub sync: Option<SyncStrategy>,
    pub max_databases: Option<u32>,
    pub map_size: Option<usize>,
}

impl Default for LmdbConfigToml {
    fn default() -> Self {
        let config = LmdbConfig::default();
        Self {
            sync: Some(config.sync),
            max_databases: Some(config.max_databases),
            map_size: Some(config.map_size),
        }
    }
}

impl From<&LmdbConfigToml> for LmdbConfig {
    fn from(toml: &LmdbConfigToml) -> Self {
        let mut config = LmdbConfig::default();

        if let Some(sync) = toml.sync {
            config.sync = sync;
        }
        if let Some(max_databases) = toml.max_databases {
            config.max_databases = max_databases;
        }
        if let Some(map_size) = toml.map_size {
            config.map_size = map_size;
        }
        config
    }
}
