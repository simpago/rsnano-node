use rsnano_store_lmdb::{LmdbConfig, SyncStrategy};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct LmdbToml {
    pub map_size: Option<usize>,
    pub max_databases: Option<u32>,
    pub sync: Option<String>,
}

impl Default for LmdbToml {
    fn default() -> Self {
        let config = LmdbConfig::default();
        (&config).into()
    }
}

impl From<&LmdbToml> for LmdbConfig {
    fn from(toml: &LmdbToml) -> Self {
        let mut config = LmdbConfig::default();

        if let Some(sync) = &toml.sync {
            config.sync = match sync.as_str() {
                "always" => SyncStrategy::Always,
                "nosync_safe" => SyncStrategy::NosyncSafe,
                "nosync_unsafe" => SyncStrategy::NosyncUnsafe,
                "nosync_unsafe_large_memory" => SyncStrategy::NosyncUnsafeLargeMemory,
                _ => panic!("Invalid sync value"),
            }
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

impl From<&LmdbConfig> for LmdbToml {
    fn from(config: &LmdbConfig) -> Self {
        Self {
            sync: Some(match config.sync {
                SyncStrategy::Always => "always".to_string(),
                SyncStrategy::NosyncSafe => "nosync_safe".to_string(),
                SyncStrategy::NosyncUnsafe => "nosync_unsafe".to_string(),
                SyncStrategy::NosyncUnsafeLargeMemory => "nosync_unsafe_large_memory".to_string(),
            }),
            max_databases: Some(config.max_databases),
            map_size: Some(config.map_size),
        }
    }
}
