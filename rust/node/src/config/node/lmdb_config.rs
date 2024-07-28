use crate::config::TomlConfigOverride;
use rsnano_store_lmdb::{LmdbConfig, SyncStrategy};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct LmdbConfigToml {
    pub sync: Option<SyncStrategy>,
    pub max_databases: Option<u32>,
    pub map_size: Option<usize>,
}

impl From<LmdbConfig> for LmdbConfigToml {
    fn from(config: LmdbConfig) -> Self {
        Self {
            sync: Some(config.sync),
            max_databases: Some(config.max_databases),
            map_size: Some(config.map_size),
        }
    }
}

impl<'de> TomlConfigOverride<'de, LmdbConfigToml> for LmdbConfig {
    fn toml_config_override(&mut self, toml: &'de LmdbConfigToml) {
        if let Some(sync) = toml.sync {
            self.sync = sync;
        }
        if let Some(max_databases) = toml.max_databases {
            self.max_databases = max_databases;
        }
        if let Some(map_size) = toml.map_size {
            self.map_size = map_size;
        }
    }
}
