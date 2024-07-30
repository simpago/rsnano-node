use super::{RpcLoggingConfig, RpcProcessConfig};
use rsnano_node::config::NetworkConstants;
use std::net::Ipv6Addr;

pub struct RpcConfigToml {
    pub address: String,
    pub port: u16,
    pub enable_control: bool,
    pub max_json_depth: u8,
    pub max_request_size: u64,
    pub rpc_logging: RpcLoggingConfig,
    pub rpc_process: RpcProcessConfig,
}

impl RpcConfigToml {
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
