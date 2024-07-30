mod rpc_config;
mod rpc_config_toml;

pub use rpc_config::*;
pub use rpc_config_toml::*;

use anyhow::Result;
use std::path::{Path, PathBuf};

pub fn get_default_rpc_filepath() -> Result<PathBuf> {
    Ok(get_default_rpc_filepath_from(
        std::env::current_exe()?.as_path(),
    ))
}

fn get_default_rpc_filepath_from(node_exe_path: &Path) -> PathBuf {
    let mut result = node_exe_path.to_path_buf();
    result.pop();
    result.push("nano_rpc");
    if let Some(ext) = node_exe_path.extension() {
        result.set_extension(ext);
    }
    result
}
