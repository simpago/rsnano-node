use std::time::Duration;

use rsnano_core::work::WorkThresholds;

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
