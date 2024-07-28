use crate::config::get_default_rpc_filepath;
use anyhow::Result;
use rsnano_core::utils::TomlWriter;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub struct RpcChildProcessConfig {
    pub enable: bool,
    pub rpc_path: PathBuf,
}

impl RpcChildProcessConfig {
    pub fn default() -> Result<Self> {
        Ok(Self {
            enable: false,
            rpc_path: get_default_rpc_filepath()?,
        })
    }
}

pub struct NodeRpcConfig {
    pub enable: bool,
    pub enable_sign_hash: bool,
    pub child_process: RpcChildProcessConfig,
}

impl NodeRpcConfig {
    pub fn default() -> Result<Self> {
        Ok(Self {
            enable: false,
            enable_sign_hash: false,
            child_process: RpcChildProcessConfig::default()?,
        })
    }

    pub fn serialize_toml(&self, toml: &mut dyn TomlWriter) -> Result<()> {
        toml.put_bool(
            "enable_sign_hash",
            self.enable_sign_hash,
            "Allow or disallow signing of hashes.\ntype:bool",
        )?;

        toml.put_child("child_process", &mut |child_process|{
            child_process.put_bool("enable", self.child_process.enable, "Enable or disable RPC child process. If false, an in-process RPC server is used.\ntype:bool")?;
            child_process.put_str("rpc_path", &self.child_process.rpc_path.to_string_lossy(), "Path to the nano_rpc executable. Must be set if child process is enabled.\ntype:string,path")?;
            Ok(())
        })?;

        Ok(())
    }
}

#[derive(Deserialize, Serialize)]
pub struct NodeRpcConfigToml {
    pub enable: Option<bool>,
    pub enable_sign_hash: Option<bool>,
    pub child_process: Option<RpcChildProcessConfigToml>,
}

impl From<NodeRpcConfig> for NodeRpcConfigToml {
    fn from(config: NodeRpcConfig) -> Self {
        Self {
            enable: Some(config.enable),
            enable_sign_hash: Some(config.enable_sign_hash),
            child_process: Some(RpcChildProcessConfigToml {
                enable: Some(config.child_process.enable),
                rpc_path: Some(config.child_process.rpc_path),
            }),
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct RpcChildProcessConfigToml {
    pub enable: Option<bool>,
    pub rpc_path: Option<PathBuf>,
}
