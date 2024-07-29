use super::NetworkConstantsDto;
use rsnano_node::{config::NetworkConstants, websocket::WebsocketConfig};
use std::convert::TryFrom;

#[repr(C)]
pub struct WebsocketConfigDto {
    pub enabled: bool,
    pub port: u16,
    pub address: [u8; 128],
    pub address_len: usize,
}

#[no_mangle]
pub unsafe extern "C" fn rsn_websocket_config_create(
    dto: *mut WebsocketConfigDto,
    network: &NetworkConstantsDto,
) -> i32 {
    let network = match NetworkConstants::try_from(network) {
        Ok(n) => n,
        Err(_) => return -1,
    };
    let websocket = WebsocketConfig::new(&network);
    let dto = &mut (*dto);
    fill_websocket_config_dto(dto, &websocket);
    0
}

pub fn fill_websocket_config_dto(dto: &mut WebsocketConfigDto, websocket: &WebsocketConfig) {
    dto.enabled = websocket.enabled;
    dto.port = websocket.port;
    let bytes = websocket.address.as_bytes();
    dto.address[..bytes.len()].copy_from_slice(bytes);
    dto.address_len = bytes.len();
}

impl From<&WebsocketConfigDto> for WebsocketConfig {
    fn from(dto: &WebsocketConfigDto) -> Self {
        Self {
            enabled: dto.enabled,
            port: dto.port,
            address: String::from_utf8_lossy(&dto.address[..dto.address_len]).to_string(),
        }
    }
}
