use crate::config::RpcChildProcessConfig;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Deserialize, Serialize)]
pub struct RpcChildProcessConfigToml {
    pub enable: Option<bool>,
    pub rpc_path: Option<PathBuf>,
}

impl RpcChildProcessConfigToml {
    pub fn new() -> Result<Self> {
        let config = RpcChildProcessConfig::new()?;
        Ok(Self {
            enable: Some(config.enable),
            rpc_path: Some(config.rpc_path),
        })
    }
}

impl From<&RpcChildProcessConfig> for RpcChildProcessConfigToml {
    fn from(config: &RpcChildProcessConfig) -> Self {
        Self {
            enable: Some(config.enable),
            rpc_path: Some(config.rpc_path.clone()),
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct NodeRpcConfigToml {
    pub enable: Option<bool>,
    pub enable_sign_hash: Option<bool>,
    pub child_process: Option<RpcChildProcessConfigToml>,
}

impl NodeRpcConfigToml {
    pub fn new() -> Result<Self> {
        Ok(Self {
            enable: Some(false),
            enable_sign_hash: Some(false),
            child_process: Some(RpcChildProcessConfigToml::new()?),
        })
    }
}
