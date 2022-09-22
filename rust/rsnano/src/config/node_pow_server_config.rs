use anyhow::Result;
use std::path::PathBuf;

use crate::utils::TomlWriter;

pub struct NodePowServerConfig {
    pub enable: bool,
    pub pow_server_path: PathBuf,
}

impl NodePowServerConfig {
    pub fn new() -> Result<Self> {
        Ok(Self {
            enable: false,
            pow_server_path: get_default_pow_server_filepath()?,
        })
    }

    pub fn serialize_toml(&self, toml: &mut dyn TomlWriter) -> Result<()> {
        toml.put_bool("enable", self.enable, "Value is currently not in use. Enable or disable starting Nano PoW Server as a child process.\ntype:bool")?;
        toml.put_str("nano_pow_server_path", &self.pow_server_path.to_string_lossy(), "Value is currently not in use. Path to the nano_pow_server executable.\ntype:string,path")?;
        Ok(())
    }
}

fn get_default_pow_server_filepath() -> Result<PathBuf> {
    let running_executable_filepath = std::env::current_exe()?;
    let mut pow_server_path = running_executable_filepath.clone();
    pow_server_path.pop();
    pow_server_path.push("nano_pow_server");
    if let Some(ext) = running_executable_filepath.extension() {
        pow_server_path.set_extension(ext);
    }
    Ok(pow_server_path)
}
