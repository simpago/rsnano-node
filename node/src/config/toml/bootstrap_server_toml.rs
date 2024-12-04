use crate::bootstrap::BootstrapServerConfig;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct BootstrapServerToml {
    pub batch_size: Option<usize>,
    pub max_queue: Option<usize>,
    pub threads: Option<usize>,
}

impl Default for BootstrapServerToml {
    fn default() -> Self {
        let config = BootstrapServerConfig::default();
        (&config).into()
    }
}

impl From<&BootstrapServerToml> for BootstrapServerConfig {
    fn from(toml: &BootstrapServerToml) -> Self {
        let mut config = BootstrapServerConfig::default();

        if let Some(max_queue) = toml.max_queue {
            config.max_queue = max_queue;
        }
        if let Some(threads) = toml.threads {
            config.threads = threads;
        }
        if let Some(batch_size) = toml.batch_size {
            config.batch_size = batch_size;
        }
        config
    }
}

impl From<&BootstrapServerConfig> for BootstrapServerToml {
    fn from(config: &BootstrapServerConfig) -> Self {
        Self {
            max_queue: Some(config.max_queue),
            threads: Some(config.threads),
            batch_size: Some(config.batch_size),
        }
    }
}
