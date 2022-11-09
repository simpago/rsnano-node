use std::os::unix::prelude::OsStrExt;

use crate::config::NodePowServerConfig;

#[repr(C)]
pub struct NodePowServerConfigDto {
    pub enable: bool,
    pub pow_server_path: [u8; 128],
    pub pow_server_path_len: usize,
}

pub fn fill_node_pow_server_config_dto(
    dto: &mut NodePowServerConfigDto,
    cfg: &NodePowServerConfig,
) {
    dto.enable = cfg.enable;
    let bytes = cfg.pow_server_path.as_os_str().as_bytes();
    dto.pow_server_path[..bytes.len()].copy_from_slice(bytes);
    dto.pow_server_path_len = bytes.len();
}

impl From<&NodePowServerConfigDto> for NodePowServerConfig {
    fn from(dto: &NodePowServerConfigDto) -> Self {
        Self {
            enable: dto.enable,
            pow_server_path: String::from_utf8_lossy(
                &dto.pow_server_path[..dto.pow_server_path_len],
            )
            .to_string()
            .into(),
        }
    }
}
