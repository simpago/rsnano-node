use crate::config::get_default_rpc_filepath;
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
