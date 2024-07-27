use crate::config::BlockProcessorConfig;
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
pub struct BlockProcessorConfigToml {
    pub max_peer_queue: Option<usize>,
    pub max_system_queue: Option<usize>,
    pub priority_live: Option<usize>,
    pub priority_bootstrap: Option<usize>,
    pub priority_local: Option<usize>,
}

impl From<BlockProcessorConfig> for BlockProcessorConfigToml {
    fn from(config: BlockProcessorConfig) -> Self {
        Self {
            max_peer_queue: Some(config.max_peer_queue),
            max_system_queue: Some(config.max_system_queue),
            priority_live: Some(config.priority_live),
            priority_bootstrap: Some(config.priority_bootstrap),
            priority_local: Some(config.priority_local),
        }
    }
}
