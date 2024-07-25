use super::get_default_rpc_filepath;
use anyhow::Result;
use rsnano_core::utils::TomlWriter;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct RpcChildProcessConfig {
    pub enable: bool,
    pub rpc_path: PathBuf,
}

impl RpcChildProcessConfig {
    pub fn new() -> Result<Self> {
        Ok(Self {
            enable: false,
            rpc_path: get_default_rpc_filepath()?,
        })
    }

    pub fn deserialize_toml(&mut self, table: &toml::value::Table) -> Result<()> {
        if let Some(enable) = table.get("enable").and_then(|v| v.as_bool()) {
            self.enable = enable;
        }
        if let Some(rpc_path) = table.get("rpc_path").and_then(|v| v.as_str()) {
            self.rpc_path = PathBuf::from(rpc_path);
        }
        Ok(())
    }
}

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

    pub fn deserialize_toml(&mut self, table: &toml::value::Table) -> Result<()> {
        if let Some(enable_sign_hash) = table.get("enable_sign_hash").and_then(|v| v.as_bool()) {
            self.enable_sign_hash = enable_sign_hash;
        }
        if let Some(child_process) = table.get("child_process").and_then(|v| v.as_table()) {
            self.child_process.deserialize_toml(child_process)?;
        }
        Ok(())
    }
}
