use rsnano_core::utils::TomlWriter;
use serde::{Deserialize, Serialize};

use crate::config::TomlConfigOverride;

#[derive(Clone, Debug)]
pub struct BootstrapServerConfig {
    pub max_queue: usize,
    pub threads: usize,
    pub batch_size: usize,
}

impl Default for BootstrapServerConfig {
    fn default() -> Self {
        Self {
            max_queue: 16,
            threads: 1,
            batch_size: 64,
        }
    }
}

impl BootstrapServerConfig {
    pub fn serialize_toml(&self, toml: &mut dyn TomlWriter) -> anyhow::Result<()> {
        toml.put_usize(
            "max_queue",
            self.max_queue,
            "Maximum number of queued requests per peer. \ntype:uint64",
        )?;
        toml.put_usize(
            "threads",
            self.threads,
            "Number of threads to process requests. \ntype:uint64",
        )?;
        toml.put_usize(
            "batch_size",
            self.batch_size,
            "Maximum number of requests to process in a single batch. \ntype:uint64",
        )
    }
}

#[derive(Deserialize, Serialize)]
pub struct BootstrapServerConfigToml {
    pub max_queue: Option<usize>,
    pub threads: Option<usize>,
    pub batch_size: Option<usize>,
}

impl From<BootstrapServerConfig> for BootstrapServerConfigToml {
    fn from(config: BootstrapServerConfig) -> Self {
        Self {
            max_queue: Some(config.max_queue),
            threads: Some(config.threads),
            batch_size: Some(config.batch_size),
        }
    }
}

impl<'de> TomlConfigOverride<'de, BootstrapServerConfigToml> for BootstrapServerConfig {
    fn toml_config_override(&mut self, toml: &'de BootstrapServerConfigToml) {
        if let Some(max_queue) = toml.max_queue {
            self.max_queue = max_queue;
        }
        if let Some(threads) = toml.threads {
            self.threads = threads;
        }
        if let Some(batch_size) = toml.batch_size {
            self.batch_size = batch_size;
        }
    }
}
