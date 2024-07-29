mod converters;
mod daemon_config_toml;
mod network_constants;
mod node_flags;
mod rpc_config_toml;

use crate::{node::NodeConfig, NetworkParams};
pub use daemon_config_toml::*;
pub use network_constants::*;
pub use node_flags::*;
pub use rpc_config_toml::*;
use rsnano_core::Networks;
use std::path::{Path, PathBuf};

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

pub fn get_opencl_toml_config_path(data_path: &Path) -> PathBuf {
    let mut rpc_toml = data_path.to_owned();
    rpc_toml.push("config-opencl.toml");
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
