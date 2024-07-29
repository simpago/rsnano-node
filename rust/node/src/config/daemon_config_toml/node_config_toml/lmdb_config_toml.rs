use anyhow::Result;
use rsnano_core::utils::TomlWriter;
use rsnano_store_lmdb::{LmdbConfig, SyncStrategy};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct LmdbConfigToml {
    pub sync: Option<SyncStrategy>,
    pub max_databases: Option<u32>,
    pub map_size: Option<usize>,
}

impl LmdbConfigToml {
    pub fn serialize_toml(&self, toml: &mut dyn TomlWriter) -> Result<()> {
        let sync_str = match self.sync {
            SyncStrategy::Always => "always",
            SyncStrategy::NosyncSafe => "nosync_safe",
            SyncStrategy::NosyncUnsafe => "nosync_unsafe",
            SyncStrategy::NosyncUnsafeLargeMemory => "nosync_unsafe_large_memory",
        };

        toml.put_str("sync", sync_str, "Sync strategy for flushing commits to the ledger database. This does not affect the wallet database.\ntype:string,{always, nosync_safe, nosync_unsafe, nosync_unsafe_large_memory}")?;
        toml.put_u32("max_databases", self.max_databases, "Maximum open lmdb databases. Increase default if more than 100 wallets is required.\nNote: external management is recommended when a large amounts of wallets are required (see https://docs.nano.org/integration-guides/key-management/).\ntype:uin32")?;
        toml.put_usize(
            "map_size",
            self.map_size,
            "Maximum ledger database map size in bytes.\ntype:uint64",
        )?;
        Ok(())
    }
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
    fn from(toml: &TomlLmdbConfig) -> Self {
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
