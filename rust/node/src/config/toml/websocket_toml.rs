use crate::websocket::WebsocketConfig;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct WebsocketToml {
    pub address: Option<String>,
    pub enable: Option<bool>,
    pub port: Option<u16>,
}

impl WebsocketConfig {
    pub fn merge_toml(&mut self, toml: &WebsocketToml) {
        if let Some(enabled) = toml.enable {
            self.enabled = enabled;
        }
        if let Some(port) = toml.port {
            self.port = port;
        }
        if let Some(address) = &toml.address {
            self.address = address.clone();
        }
    }
}

impl From<&WebsocketConfig> for WebsocketToml {
    fn from(websocket_config: &WebsocketConfig) -> Self {
        Self {
            enable: Some(websocket_config.enabled),
            port: Some(websocket_config.port),
            address: Some(websocket_config.address.clone()),
        }
    }
}
