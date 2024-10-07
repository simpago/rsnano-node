use crate::block_processing::BlockProcessorConfig;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct BlockProcessorToml {
    pub max_peer_queue: Option<usize>,
    pub max_system_queue: Option<usize>,
    pub priority_bootstrap: Option<usize>,
    pub priority_live: Option<usize>,
    pub priority_local: Option<usize>,
}

impl From<&BlockProcessorConfig> for BlockProcessorToml {
    fn from(config: &BlockProcessorConfig) -> Self {
        Self {
            max_peer_queue: Some(config.max_peer_queue),
            max_system_queue: Some(config.max_system_queue),
            priority_live: Some(config.priority_live),
            priority_bootstrap: Some(config.priority_bootstrap),
            priority_local: Some(config.priority_local),
        }
    }
}

impl BlockProcessorConfig {
    pub fn merge_toml(&mut self, toml: &BlockProcessorToml) {
        if let Some(max_peer_queue) = toml.max_peer_queue {
            self.max_peer_queue = max_peer_queue;
        }
        if let Some(max_system_queue) = toml.max_system_queue {
            self.max_system_queue = max_system_queue;
        }
        if let Some(priority_live) = toml.priority_live {
            self.priority_live = priority_live;
        }
        if let Some(priority_local) = toml.priority_local {
            self.priority_local = priority_local;
        }
        if let Some(priority_bootstrap) = toml.priority_bootstrap {
            self.priority_bootstrap = priority_bootstrap;
        }
    }
}
