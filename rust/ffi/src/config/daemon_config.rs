use super::{
    fill_node_config_dto, fill_node_rpc_config_dto, fill_opencl_config_dto, NodeConfigDto,
    NodeRpcConfigDto, OpenclConfigDto,
};
use crate::{secure::NetworkParamsDto, utils::FfiToml};
use rsnano_core::utils::get_cpu_count;
use rsnano_node::{
    config::{DaemonConfigToml, NodeConfig},
    NetworkParams,
};
use std::{
    convert::{TryFrom, TryInto},
    ffi::c_void,
};

#[repr(C)]
pub struct DaemonConfigDto {
    pub node: NodeConfigDto,
    pub opencl: OpenclConfigDto,
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
    let cfg = match DaemonConfigToml::new(&network_params, get_cpu_count()) {
        Ok(d) => d,
        Err(_) => return -1,
    };
    let dto = &mut (*dto);
    //dto.rpc.rpc_enable = cfg.rpc.enable;
    fill_node_config_dto(&mut dto.node, &cfg.node.unwrap().into());
    fill_opencl_config_dto(&mut dto.opencl, &cfg.opencl.unwrap());
    fill_node_rpc_config_dto(&mut dto.rpc, &cfg.rpc.unwrap());
    //dto.opencl.opencl_enable = cfg.opencl.enable;
    0
}

#[no_mangle]
pub extern "C" fn rsn_daemon_config_serialize_toml(
    dto: &DaemonConfigDto,
    toml: *mut c_void,
) -> i32 {
    let mut toml = FfiToml::new(toml);
    let cfg = match DaemonConfigToml::try_from(dto) {
        Ok(d) => d,
        Err(_) => return -1,
    };
    0
    /*match cfg.serialize_toml(&mut toml) {
    Ok(_) => 0,
    Err(_) => -1,
    }*/
}

impl TryFrom<&DaemonConfigDto> for DaemonConfigToml {
    type Error = anyhow::Error;

    fn try_from(dto: &DaemonConfigDto) -> Result<Self, Self::Error> {
        let node_config: NodeConfig = (&dto.node).try_into()?;
        let result = Self {
            node: Some((&node_config).into()),
            opencl: Some((&dto.opencl).into()),
            rpc: Some((&dto.rpc).into()),
        };
        Ok(result)
    }
}
