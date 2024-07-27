use crate::config::WebsocketConfig;
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
pub struct WebsocketConfigToml {
    pub enabled: bool,
    pub port: u16,
    pub address: String,
}

impl From<WebsocketConfig> for WebsocketConfigToml {
    fn from(websocket_config: WebsocketConfig) -> Self {
        Self {
            enabled: websocket_config.enabled,
            port: websocket_config.port,
            address: websocket_config.address,
        }
    }
}
