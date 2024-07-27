use crate::config::NodeRpcConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Deserialize, Serialize)]
pub struct NodeRpcConfigToml {
    pub enable: bool,
    pub enable_sign_hash: bool,
    pub child_process: RpcChildProcessConfigToml,
}

impl From<NodeRpcConfig> for NodeRpcConfigToml {
    fn from(config: NodeRpcConfig) -> Self {
        Self {
            enable: config.enable,
            enable_sign_hash: config.enable_sign_hash,
            child_process: RpcChildProcessConfigToml {
                enable: config.child_process.enable,
                rpc_path: config.child_process.rpc_path,
            },
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct RpcChildProcessConfigToml {
    pub enable: bool,
    pub rpc_path: PathBuf,
}
