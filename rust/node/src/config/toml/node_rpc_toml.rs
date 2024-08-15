use crate::config::{DaemonConfig, NodeRpcConfig, RpcChildProcessConfig};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Deserialize, Serialize)]
pub struct RpcChildProcessToml {
    pub enable: Option<bool>,
    pub rpc_path: Option<PathBuf>,
}

impl RpcChildProcessToml {
    pub fn new() -> Self {
        let config = RpcChildProcessConfig::new();
        Self {
            enable: Some(config.enable),
            rpc_path: Some(config.rpc_path),
        }
    }
}

impl From<&RpcChildProcessConfig> for RpcChildProcessToml {
    fn from(config: &RpcChildProcessConfig) -> Self {
        Self {
            enable: Some(config.enable),
            rpc_path: Some(config.rpc_path.clone()),
        }
    }
}

impl From<&RpcChildProcessToml> for RpcChildProcessConfig {
    fn from(toml: &RpcChildProcessToml) -> Self {
        let mut config = RpcChildProcessConfig::new();
        if let Some(enable) = toml.enable {
            config.enable = enable;
        }
        if let Some(rpc_path) = &toml.rpc_path {
            config.rpc_path = rpc_path.clone();
        }
        config
    }
}

#[derive(Deserialize, Serialize)]
pub struct NodeRpcToml {
    pub enable: Option<bool>,
    pub enable_sign_hash: Option<bool>,
    pub child_process: Option<RpcChildProcessToml>,
}

impl Default for NodeRpcToml {
    fn default() -> Self {
        let config = DaemonConfig::default();
        (&config).into()
    }
}

impl From<&NodeRpcToml> for NodeRpcConfig {
    fn from(toml: &NodeRpcToml) -> Self {
        let mut config = NodeRpcConfig::default();
        if let Some(enable_sign_hash) = toml.enable_sign_hash {
            config.enable_sign_hash = enable_sign_hash;
        }
        if let Some(child_process) = &toml.child_process {
            config.child_process = child_process.into();
        }

        config
    }
}
