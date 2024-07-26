use rsnano_core::utils::TomlWriter;
use serde::{Deserialize, Serialize};

use crate::config::BlockProcessorConfig;

#[derive(Clone, Deserialize, Serialize)]
pub struct BlockProcessorConfigToml {
    // Maximum number of blocks to queue from network peers
    pub max_peer_queue: usize,
    // Maximum number of blocks to queue from system components (local RPC, bootstrap)
    pub max_system_queue: usize,

    // Higher priority gets processed more frequently
    pub priority_live: usize,
    pub priority_bootstrap: usize,
    pub priority_local: usize,
}

impl From<BlockProcessorConfig> for BlockProcessorConfigToml {
    fn from(config: BlockProcessorConfig) -> Self {
        Self {
            max_peer_queue: config.max_peer_queue,
            max_system_queue: config.max_system_queue,
            priority_live: config.priority_live,
            priority_bootstrap: config.priority_bootstrap,
            priority_local: config.priority_local,
        }
    }
}

impl BlockProcessorConfigToml {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn serialize_toml(&self, toml: &mut dyn TomlWriter) -> anyhow::Result<()> {
        toml.put_usize(
            "max_peer_queue",
            self.max_peer_queue,
            "Maximum number of blocks to queue from network peers. \ntype:uint64",
        )?;
        toml.put_usize("max_system_queue", self.max_system_queue, "Maximum number of blocks to queue from system components (local RPC, bootstrap). \ntype:uint64")?;
        toml.put_usize("priority_live", self.priority_live, "Priority for live network blocks. Higher priority gets processed more frequently. \ntype:uint64")?;
        toml.put_usize("priority_bootstrap", self.priority_bootstrap, "Priority for bootstrap blocks. Higher priority gets processed more frequently. \ntype:uint64")?;
        toml.put_usize("priority_local", self.priority_local, "Priority for local RPC blocks. Higher priority gets processed more frequently. \ntype:uint64")
    }
}
