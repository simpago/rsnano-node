use super::get_default_rpc_filepath;
use anyhow::Result;
use rsnano_core::utils::TomlWriter;
use std::path::PathBuf;

pub struct RpcChildProcessConfigToml {
    pub enable: bool,
    pub rpc_path: PathBuf,
}

impl RpcChildProcessConfigToml {
    pub fn new() -> Result<Self> {
        Ok(Self {
            enable: false,
            rpc_path: get_default_rpc_filepath()?,
        })
    }
}

pub struct RpcConfigToml {
    pub enable_sign_hash: bool,
    pub child_process: RpcChildProcessConfigToml,
}

impl RpcConfigToml {
    pub fn new() -> Result<Self> {
        Ok(Self {
            enable_sign_hash: false,
            child_process: RpcChildProcessConfigToml::new()?,
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
