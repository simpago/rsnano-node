mod block_processor;
mod bootstrap_ascending;
mod converters;
mod daemon_config;
mod diagnostics_config;
mod network_constants;
mod node_config;
mod node_flags;
mod node_rpc_config;
mod opencl_config;
mod optimistic_scheduler_config;
mod rpc_config;
mod websocket_config;

use crate::NetworkParams;
pub use block_processor::*;
pub use bootstrap_ascending::*;
pub use daemon_config::*;
pub use diagnostics_config::*;
pub use network_constants::*;
pub use node_config::*;
pub use node_flags::*;
pub use node_rpc_config::*;
pub use opencl_config::*;
pub use optimistic_scheduler_config::*;
pub use rpc_config::*;
use rsnano_core::Networks;
use std::path::{Path, PathBuf};
pub use websocket_config::*;

pub fn get_node_toml_config_path(data_path: &Path) -> PathBuf {
    let mut node_toml = data_path.to_owned();
    node_toml.push("config-node.toml");
    node_toml
}

pub fn get_rpc_toml_config_path(data_path: &Path) -> PathBuf {
    let mut rpc_toml = data_path.to_owned();
    rpc_toml.push("config-rpc.toml");
    rpc_toml
}

pub fn force_nano_dev_network() {
    NetworkConstants::set_active_network(Networks::NanoDevNetwork);
}

pub struct GlobalConfig {
    pub node_config: NodeConfig,
    pub flags: NodeFlags,
    pub network_params: NetworkParams,
}
