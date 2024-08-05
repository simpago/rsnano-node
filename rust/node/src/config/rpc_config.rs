use super::NetworkConstants;
use anyhow::Result;
use rsnano_core::utils::{get_cpu_count, TomlWriter};
use std::{
    net::Ipv6Addr,
    path::{Path, PathBuf},
};

pub fn get_default_rpc_filepath() -> PathBuf {
    get_default_rpc_filepath_from(std::env::current_exe().unwrap_or_default().as_path())
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

pub struct RpcConfig {
    pub address: String,
    pub port: u16,
    pub enable_control: bool,
    pub max_json_depth: u8,
    pub max_request_size: u64,
    pub rpc_logging: RpcLoggingConfig,
    pub rpc_process: RpcProcessConfig,
}

impl RpcConfig {
    pub fn new(network_constants: &NetworkConstants, parallelism: usize) -> Self {
        Self::new2(
            network_constants,
            parallelism,
            network_constants.default_rpc_port,
            false,
        )
    }

    pub fn new2(
        network_constants: &NetworkConstants,
        parallelism: usize,
        port: u16,
        enable_control: bool,
    ) -> Self {
        Self {
            address: Ipv6Addr::LOCALHOST.to_string(),
            port,
            enable_control,
            max_json_depth: 20,
            max_request_size: 32 * 1024 * 1024,
            rpc_logging: RpcLoggingConfig::new(),
            rpc_process: RpcProcessConfig::new(network_constants, parallelism),
        }
    }

    pub fn serialize_toml(&self, toml: &mut dyn TomlWriter) -> Result<()> {
        toml.put_str(
            "address",
            &self.address,
            "Bind address for the RPC server.\ntype:string,ip",
        )?;
        toml.put_u16(
            "port",
            self.port,
            "Listening port for the RPC server.\ntype:uint16",
        )?;
        toml.put_bool("enable_control", self.enable_control, "Enable or disable control-level requests.\nWARNING: Enabling this gives anyone with RPC access the ability to stop the node and access wallet funds.\ntype:bool")?;
        toml.put_u16(
            "max_json_depth",
            self.max_json_depth as u16,
            "Maximum number of levels in JSON requests.\ntype:uint8",
        )?;
        toml.put_u64(
            "max_request_size",
            self.max_request_size,
            "Maximum number of bytes allowed in request bodies.\ntype:uint64",
        )?;

        toml.put_child("process", &mut |rpc_process| {
            rpc_process.put_u32(
                "io_threads",
                self.rpc_process.io_threads,
                "Number of threads used to serve IO.\ntype:uint32",
            )?;
            rpc_process.put_str(
                "ipc_address",
                &self.rpc_process.ipc_address,
                "Address of IPC server.\ntype:string,ip",
            )?;
            rpc_process.put_u16(
                "ipc_port",
                self.rpc_process.ipc_port,
                "Listening port of IPC server.\ntype:uint16",
            )?;
            rpc_process.put_u32(
                "num_ipc_connections",
                self.rpc_process.num_ipc_connections,
                "Number of IPC connections to establish.\ntype:uint32",
            )?;
            Ok(())
        })?;

        toml.put_child("logging", &mut |rpc_logging| {
            rpc_logging.put_bool(
                "log_rpc",
                self.rpc_logging.log_rpc,
                "Whether to log RPC calls.\ntype:bool",
            )?;
            Ok(())
        })?;
        Ok(())
    }
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self::new(&NetworkConstants::default(), get_cpu_count())
    }
}

pub struct RpcLoggingConfig {
    pub log_rpc: bool,
}

impl Default for RpcLoggingConfig {
    fn default() -> Self {
        Self { log_rpc: true }
    }
}

impl RpcLoggingConfig {
    pub fn new() -> Self {
        Default::default()
    }
}

pub struct RpcProcessConfig {
    pub io_threads: u32,
    pub ipc_address: String,
    pub ipc_port: u16,
    pub num_ipc_connections: u32,
}

impl RpcProcessConfig {
    pub fn new(network_constants: &NetworkConstants, parallelism: usize) -> Self {
        Self {
            io_threads: if parallelism > 4 {
                parallelism as u32
            } else {
                4
            },
            ipc_address: Ipv6Addr::LOCALHOST.to_string(),
            ipc_port: network_constants.default_ipc_port,
            num_ipc_connections: if network_constants.is_live_network()
                || network_constants.is_test_network()
            {
                8
            } else if network_constants.is_beta_network() {
                4
            } else {
                1
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use rsnano_node::config::get_default_rpc_filepath_from;
    use std::path::Path;

    #[test]
    fn default_rpc_filepath() -> Result<()> {
        assert_eq!(
            get_default_rpc_filepath_from(Path::new("/path/to/nano_node")),
            Path::new("/path/to/nano_rpc")
        );

        assert_eq!(
            get_default_rpc_filepath_from(Path::new("/nano_node")),
            Path::new("/nano_rpc")
        );

        assert_eq!(
            get_default_rpc_filepath_from(Path::new("/bin/nano_node.exe")),
            Path::new("/bin/nano_rpc.exe")
        );

        Ok(())
    }
}
