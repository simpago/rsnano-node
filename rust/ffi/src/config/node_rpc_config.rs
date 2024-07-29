use rsnano_node::config::{NodeRpcConfigToml, RpcChildProcessConfigToml};
use std::os::unix::prelude::OsStrExt;

#[repr(C)]
pub struct NodeRpcConfigDto {
    pub rpc_enable: bool,
    pub rpc_path: [u8; 512],
    pub rpc_path_length: usize,
    pub enable_child_process: bool,
    pub enable_sign_hash: bool,
}

#[no_mangle]
pub unsafe extern "C" fn rsn_node_rpc_config_create(dto: *mut NodeRpcConfigDto) -> i32 {
    let config = match NodeRpcConfigToml::new() {
        Ok(c) => c,
        Err(_) => return -1,
    };

    let dto = &mut (*dto);
    fill_node_rpc_config_dto(dto, &config);
    0
}

pub fn fill_node_rpc_config_dto(dto: &mut NodeRpcConfigDto, config: &NodeRpcConfigToml) {
    dto.enable_sign_hash = config.enable_sign_hash.unwrap();
    dto.enable_child_process = config.child_process.as_ref().unwrap().enable.unwrap();
    let bytes: &[u8] = config
        .child_process
        .as_ref()
        .unwrap()
        .rpc_path
        .as_ref()
        .unwrap()
        .as_os_str()
        .as_bytes();
    dto.rpc_path[..bytes.len()].copy_from_slice(bytes);
    dto.rpc_path_length = bytes.len();
}

impl From<&NodeRpcConfigDto> for NodeRpcConfigToml {
    fn from(dto: &NodeRpcConfigDto) -> Self {
        Self {
            enable: Some(dto.rpc_enable),
            enable_sign_hash: Some(dto.enable_sign_hash),
            child_process: Some(RpcChildProcessConfigToml {
                enable: Some(dto.enable_child_process),
                rpc_path: Some(
                    String::from_utf8_lossy(&dto.rpc_path[..dto.rpc_path_length])
                        .to_string()
                        .into(),
                ),
            }),
        }
    }
}
