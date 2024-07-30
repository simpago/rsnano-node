use super::{
    fill_node_config_dto, fill_node_rpc_config_dto, fill_opencl_config_dto, NodeConfigDto,
    NodeRpcConfigDto, OpenclConfigDto,
};
use crate::{
    secure::NetworkParamsDto,
    utils::{FfiToml, FfiTomlArrayWriter},
};
use rsnano_core::utils::{get_cpu_count, TomlArrayWriter, TomlWriter};
use rsnano_node::{
    config::{DaemonConfig, DaemonConfigToml},
    NetworkParams,
};
use std::{
    convert::{TryFrom, TryInto},
    ffi::c_void,
};
use tracing::debug;

#[repr(C)]
pub struct DaemonConfigDto {
    pub rpc_enable: bool,
    pub node: NodeConfigDto,
    pub opencl: OpenclConfigDto,
    pub opencl_enable: bool,
    pub rpc: NodeRpcConfigDto,
}

#[no_mangle]
pub unsafe extern "C" fn rsn_daemon_config_create(
    dto: *mut DaemonConfigDto,
    network_params: &NetworkParamsDto,
) -> i32 {
    let network_params = match NetworkParams::try_from(network_params) {
        Ok(n) => n,
        Err(_) => return -1,
    };
    let cfg = match DaemonConfig::new(&network_params, get_cpu_count()) {
        Ok(d) => d,
        Err(_) => return -1,
    };
    let dto = &mut (*dto);
    dto.rpc_enable = cfg.rpc_enable;
    fill_node_config_dto(&mut dto.node, &cfg.node);
    fill_opencl_config_dto(&mut dto.opencl, &cfg.opencl);
    fill_node_rpc_config_dto(&mut dto.rpc, &cfg.rpc);
    dto.opencl_enable = cfg.opencl_enable;
    0
}

#[no_mangle]
pub extern "C" fn rsn_daemon_config_serialize_toml(
    dto: &DaemonConfigDto,
    toml: *mut c_void,
) -> i32 {
    let mut ffi_toml = FfiTomlArrayWriter::new(toml);
    let cfg = match DaemonConfig::try_from(dto) {
        Ok(d) => d,
        Err(_) => return -1,
    };
    let toml_config: DaemonConfigToml = cfg.into();
    match toml::to_string(&toml_config) {
        Ok(toml_string) => {
            //ffi_toml.push_back_str(&toml_string).unwrap();
            0
        }
        Err(_) => -1,
    }
}

impl TryFrom<&DaemonConfigDto> for DaemonConfig {
    type Error = anyhow::Error;

    fn try_from(dto: &DaemonConfigDto) -> Result<Self, Self::Error> {
        let result = Self {
            rpc_enable: dto.rpc_enable,
            node: (&dto.node).try_into()?,
            opencl: (&dto.opencl).into(),
            opencl_enable: dto.opencl_enable,
            rpc: (&dto.rpc).into(),
        };
        Ok(result)
    }
}
