use super::NetworkConstants;
use rsnano_core::utils::get_cpu_count;
use std::net::Ipv6Addr;

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
