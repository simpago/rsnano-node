use crate::block_processing::BlockProcessorConfig;

#[derive(Clone)]
pub struct BlockProcessorToml {
    // Maximum number of blocks to queue from network peers
    pub max_peer_queue: usize,
    // Maximum number of blocks to queue from system components (local RPC, bootstrap)
    pub max_system_queue: usize,

    // Higher priority gets processed more frequently
    pub priority_live: usize,
    pub priority_bootstrap: usize,
    pub priority_local: usize,
}

impl Default for BlockProcessorToml {
    fn default() -> Self {
        let config = BlockProcessorConfig::default();
        Self {
            max_peer_queue: config.max_peer_queue,
            max_system_queue: config.max_system_queue,
            priority_live: config.priority_live,
            priority_bootstrap: config.priority_bootstrap,
            priority_local: config.priority_local,
        }
    }
}

impl BlockProcessorToml {
    pub fn new() -> Self {
        Default::default()
    }
}
