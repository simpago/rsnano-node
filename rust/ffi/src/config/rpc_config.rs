use super::NetworkConstantsDto;
use crate::utils::FfiToml;
use rsnano_core::utils::get_cpu_count;
use rsnano_node::config::{NetworkConstants, RpcConfig, RpcLoggingConfig, RpcProcessConfig};
use std::{convert::TryFrom, ffi::c_void};

#[repr(C)]
pub struct RpcConfigDto {
    pub address: [u8; 128],
    pub address_len: usize,
    pub port: u16,
    pub enable_control: bool,
    pub max_json_depth: u8,
    pub max_request_size: u64,
    pub rpc_log: bool,
    pub rpc_process: RpcProcessConfigDto,
}

#[repr(C)]
pub struct RpcProcessConfigDto {
    pub io_threads: u32,
    pub ipc_address: [u8; 128],
    pub ipc_address_len: usize,
    pub ipc_port: u16,
    pub num_ipc_connections: u32,
}

#[no_mangle]
pub unsafe extern "C" fn rsn_rpc_config_create(
    dto: *mut RpcConfigDto,
    network_constants: &NetworkConstantsDto,
) -> i32 {
    let network_constants = match NetworkConstants::try_from(network_constants) {
        Ok(nc) => nc,
        Err(_) => return -1,
    };
    let cfg = RpcConfig::new(&network_constants, get_cpu_count());
    let dto = &mut (*dto);
    fill_rpc_config_dto(dto, &cfg);
    0
}

#[no_mangle]
pub unsafe extern "C" fn rsn_rpc_config_create2(
    dto: *mut RpcConfigDto,
    network_constants: &NetworkConstantsDto,
    port: u16,
    enable_control: bool,
) -> i32 {
    let network_constants = match NetworkConstants::try_from(network_constants) {
        Ok(nc) => nc,
        Err(_) => return -1,
    };
    let cfg = RpcConfig::new2(&network_constants, get_cpu_count(), port, enable_control);
    let dto = &mut (*dto);
    fill_rpc_config_dto(dto, &cfg);
    0
}

fn fill_rpc_config_dto(dto: &mut RpcConfigDto, cfg: &RpcConfig) {
    let bytes = cfg.address.as_bytes();
    dto.address[..bytes.len()].copy_from_slice(bytes);
    dto.address_len = bytes.len();
    dto.port = cfg.port;
    dto.enable_control = cfg.enable_control;
    dto.max_json_depth = cfg.max_json_depth;
    dto.max_request_size = cfg.max_request_size;
    dto.rpc_log = cfg.rpc_logging.log_rpc;
    dto.rpc_process.io_threads = cfg.rpc_process.io_threads;
    let bytes = cfg.rpc_process.ipc_address.as_bytes();
    dto.rpc_process.ipc_address[..bytes.len()].copy_from_slice(bytes);
    dto.rpc_process.ipc_address_len = bytes.len();
    dto.rpc_process.ipc_port = cfg.rpc_process.ipc_port;
    dto.rpc_process.num_ipc_connections = cfg.rpc_process.num_ipc_connections;
}

#[no_mangle]
pub extern "C" fn rsn_rpc_config_serialize_toml(dto: &RpcConfigDto, toml: *mut c_void) -> i32 {
    let cfg = match RpcConfig::try_from(dto) {
        Ok(c) => c,
        Err(_) => return -1,
    };
    let mut toml = FfiToml::new(toml);
    match cfg.serialize_toml(&mut toml) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

impl TryFrom<&RpcConfigDto> for RpcConfig {
    type Error = anyhow::Error;

    fn try_from(dto: &RpcConfigDto) -> Result<Self, Self::Error> {
        let cfg = RpcConfig {
            address: String::from_utf8_lossy(&dto.address[..dto.address_len]).to_string(),
            port: dto.port,
            enable_control: dto.enable_control,
            max_json_depth: dto.max_json_depth,
            max_request_size: dto.max_request_size,
            rpc_logging: RpcLoggingConfig {
                log_rpc: dto.rpc_log,
            },
            rpc_process: RpcProcessConfig {
                io_threads: dto.rpc_process.io_threads,
                ipc_address: String::from_utf8_lossy(
                    &dto.rpc_process.ipc_address[..dto.rpc_process.ipc_address_len],
                )
                .to_string(),
                ipc_port: dto.rpc_process.ipc_port,
                num_ipc_connections: dto.rpc_process.num_ipc_connections,
            },
        };
        Ok(cfg)
    }
}
