mod bootstrap_config;
mod daemon_config;
mod diagnostics_config;
mod lmdb_config;
mod network_constants;
mod node_config;
mod node_flags;
mod node_rpc_config;
mod opencl_config;
mod optimistic_scheduler_config;
mod rpc_config;
mod websocket_config;

use std::time::Duration;

pub use diagnostics_config::*;
pub use lmdb_config::LmdbConfigDto;
pub use network_constants::*;
pub use node_config::*;
pub use node_flags::NodeFlagsHandle;
pub use node_rpc_config::*;
pub use opencl_config::*;
pub use optimistic_scheduler_config::*;
pub use rpc_config::*;
use rsnano_core::work::WorkThresholds;
use rsnano_node::config::{BlockProcessorConfig, BlockProcessorConfigToml};
pub use websocket_config::*;

#[repr(C)]
pub struct BlockProcessorConfigDto {
    pub max_peer_queue: usize,
    pub max_system_queue: usize,
    pub priority_live: usize,
    pub priority_bootstrap: usize,
    pub priority_local: usize,
}

impl From<&BlockProcessorConfig> for BlockProcessorConfigDto {
    fn from(value: &BlockProcessorConfig) -> Self {
        Self {
            max_peer_queue: value.max_peer_queue,
            max_system_queue: value.max_system_queue,
            priority_live: value.priority_live,
            priority_bootstrap: value.priority_bootstrap,
            priority_local: value.priority_local,
        }
    }
}

impl From<&BlockProcessorConfigDto> for BlockProcessorConfig {
    fn from(value: &BlockProcessorConfigDto) -> Self {
        Self {
            max_peer_queue: value.max_peer_queue,
            max_system_queue: value.max_system_queue,
            priority_live: value.priority_live,
            priority_bootstrap: value.priority_bootstrap,
            priority_local: value.priority_local,
            batch_max_time: Duration::new(0, 0),
            full_size: 0,
            batch_size: 0,
            work_thresholds: WorkThresholds::new(0, 0, 0),
        }
    }
}

impl From<&BlockProcessorConfigDto> for BlockProcessorConfigToml {
    fn from(value: &BlockProcessorConfigDto) -> Self {
        Self {
            max_peer_queue: Some(value.max_peer_queue),
            max_system_queue: Some(value.max_system_queue),
            priority_live: Some(value.priority_live),
            priority_bootstrap: Some(value.priority_bootstrap),
            priority_local: Some(value.priority_local),
        }
    }
}

impl From<&BlockProcessorConfigToml> for BlockProcessorConfigDto {
    fn from(value: &BlockProcessorConfigToml) -> Self {
        Self {
            max_peer_queue: value.max_peer_queue.unwrap(),
            max_system_queue: value.max_system_queue.unwrap(),
            priority_live: value.priority_live.unwrap(),
            priority_bootstrap: value.priority_bootstrap.unwrap(),
            priority_local: value.priority_local.unwrap(),
        }
    }
}
