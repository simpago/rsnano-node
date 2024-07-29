use crate::config::get_default_rpc_filepath;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub struct NodeRpcConfig {
    pub enable_sign_hash: bool,
    pub child_process: RpcChildProcessConfig,
}

impl NodeRpcConfig {
    pub fn new() -> Result<Self> {
        Ok(Self {
            enable_sign_hash: false,
            child_process: RpcChildProcessConfig::new()?,
        })
    }
}

pub struct RpcChildProcessConfig {
    pub enable: bool,
    pub rpc_path: PathBuf,
}

impl From<&RpcChildProcessConfig> for RpcChildProcessConfigToml {
    fn from(config: &RpcChildProcessConfig) -> Self {
        Self {
            enable: Some(config.enable),
            rpc_path: Some(config.rpc_path.clone()),
        }
    }
}

impl RpcChildProcessConfig {
    pub fn new() -> Result<Self> {
        Ok(Self {
            enable: false,
            rpc_path: get_default_rpc_filepath()?,
        })
    }
}

#[derive(Deserialize, Serialize)]
pub struct RpcChildProcessConfigToml {
    pub enable: Option<bool>,
    pub rpc_path: Option<PathBuf>,
}

impl RpcChildProcessConfigToml {
    pub fn new() -> Result<Self> {
        Ok(Self {
            enable: Some(false),
            rpc_path: Some(get_default_rpc_filepath()?),
        })
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
