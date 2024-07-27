use rsnano_core::{utils::TomlWriter, work::WorkThresholds};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone)]
pub struct BlockProcessorConfig {
    // Maximum number of blocks to queue from network peers
    pub max_peer_queue: usize,
    //
    // Maximum number of blocks to queue from system components (local RPC, bootstrap)
    pub max_system_queue: usize,

    // Higher priority gets processed more frequently
    pub priority_live: usize,
    pub priority_bootstrap: usize,
    pub priority_local: usize,
    pub batch_max_time: Duration,
    pub full_size: usize,
    pub batch_size: usize,
    pub work_thresholds: WorkThresholds,
}

impl Default for BlockProcessorConfig {
    fn default() -> Self {
        Self {
            max_peer_queue: 128,
            max_system_queue: 16 * 1024,
            priority_live: 1,
            priority_bootstrap: 8,
            priority_local: 16,
            batch_max_time: Duration::from_millis(500),
            full_size: 65536,
            batch_size: 0,
            work_thresholds: WorkThresholds::default(),
        }
    }
}

impl BlockProcessorConfig {
    pub(crate) fn config_toml_override(&mut self, toml: &BlockProcessorConfigToml) {
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

#[derive(Deserialize, Serialize)]
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
