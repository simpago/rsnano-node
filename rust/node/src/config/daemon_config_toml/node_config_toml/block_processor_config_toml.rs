use crate::block_processing::BlockProcessorConfig;
use rsnano_core::utils::TomlWriter;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct BlockProcessorConfigToml {
    pub max_peer_queue: Option<usize>,
    pub max_system_queue: Option<usize>,
    pub priority_live: Option<usize>,
    pub priority_bootstrap: Option<usize>,
    pub priority_local: Option<usize>,
}

impl BlockProcessorConfigToml {
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

impl Default for BlockProcessorConfigToml {
    fn default() -> Self {
        let config = BlockProcessorConfig::default();
        Self {
            max_peer_queue: Some(config.max_peer_queue),
            max_system_queue: Some(config.max_system_queue),
            priority_live: Some(config.priority_live),
            priority_bootstrap: Some(config.priority_bootstrap),
            priority_local: Some(config.priority_local),
        }
    }
}

impl From<&BlockProcessorConfigToml> for BlockProcessorConfig {
    fn from(toml: &BlockProcessorConfigToml) -> Self {
        let mut config = BlockProcessorConfig::default();

        if let Some(max_peer_queue) = toml.max_peer_queue {
            config.max_peer_queue = max_peer_queue;
        }
        if let Some(max_system_queue) = toml.max_system_queue {
            config.max_system_queue = max_system_queue;
        }
        if let Some(priority_live) = toml.priority_live {
            config.priority_live = priority_live;
        }
        if let Some(priority_local) = toml.priority_local {
            config.priority_local = priority_local;
        }
        if let Some(priority_bootstrap) = toml.priority_bootstrap {
            config.priority_bootstrap = priority_bootstrap;
        }

        config
    }
}
