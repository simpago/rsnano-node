use super::{
    fill_node_config_dto, fill_node_rpc_config_dto, fill_opencl_config_dto, NodeConfigDto,
    NodeRpcConfigDto, OpenclConfigDto,
};
use crate::secure::NetworkParamsDto;
use rsnano_core::utils::get_cpu_count;
use rsnano_node::{
    config::{DaemonConfig, DaemonToml},
    NetworkParams,
};
use std::{
    convert::{TryFrom, TryInto},
    ffi::{c_char, CString},
    ptr::copy_nonoverlapping,
};

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
    let cfg = DaemonConfig::new(&network_params, get_cpu_count());
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
    buffer: *mut c_char,
    buffer_len: usize,
) -> i32 {
    let cfg = match DaemonConfig::try_from(dto) {
        Ok(d) => d,
        Err(_) => return -1,
    };

    let toml: DaemonToml = (&cfg).into();
    let toml_str = match toml::to_string(&toml) {
        Ok(t) => t,
        Err(_) => return -1,
    };

    let c_string = match CString::new(toml_str) {
        Ok(c) => c,
        Err(_) => return -1,
    };

    let toml_len = c_string.as_bytes_with_nul().len();

    if toml_len > buffer_len {
        return -1;
    }

    unsafe {
        copy_nonoverlapping(c_string.as_ptr(), buffer, toml_len);
    }
    0
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
