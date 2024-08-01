use crate::config::{RpcConfig, RpcLoggingConfig, RpcProcessConfig};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct RpcToml {
    pub address: Option<String>,
    pub port: Option<u16>,
    pub enable_control: Option<bool>,
    pub max_json_depth: Option<u8>,
    pub max_request_size: Option<u64>,
    pub rpc_logging: Option<RpcLoggingToml>,
    pub rpc_process: Option<RpcProcessToml>,
}

impl Default for RpcToml {
    fn default() -> Self {
        let config = RpcConfig::default();
        Self {
            address: Some(config.address),
            port: Some(config.port),
            enable_control: Some(config.enable_control),
            max_json_depth: Some(config.max_json_depth),
            max_request_size: Some(config.max_request_size),
            rpc_logging: Some((&config.rpc_logging).into()),
            rpc_process: Some((&config.rpc_process).into()),
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct RpcLoggingToml {
    pub log_rpc: Option<bool>,
}

#[derive(Deserialize, Serialize)]
pub struct RpcProcessToml {
    pub io_threads: Option<u32>,
    pub ipc_address: Option<String>,
    pub ipc_port: Option<u16>,
    pub num_ipc_connections: Option<u32>,
}

impl From<&RpcConfig> for RpcToml {
    fn from(config: &RpcConfig) -> Self {
        RpcToml {
            address: Some(config.address.clone()),
            port: Some(config.port),
            enable_control: Some(config.enable_control),
            max_json_depth: Some(config.max_json_depth),
            max_request_size: Some(config.max_request_size),
            rpc_logging: Some((&config.rpc_logging).into()),
            rpc_process: Some((&config.rpc_process).into()),
        }
    }
}

impl From<&RpcLoggingConfig> for RpcLoggingToml {
    fn from(config: &RpcLoggingConfig) -> Self {
        RpcLoggingToml {
            log_rpc: Some(config.log_rpc),
        }
    }
}

impl From<&RpcProcessConfig> for RpcProcessToml {
    fn from(config: &RpcProcessConfig) -> Self {
        RpcProcessToml {
            io_threads: Some(config.io_threads),
            ipc_address: Some(config.ipc_address.clone()),
            ipc_port: Some(config.ipc_port),
            num_ipc_connections: Some(config.num_ipc_connections),
        }
    }
}
