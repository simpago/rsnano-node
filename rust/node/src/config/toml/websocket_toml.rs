use crate::websocket::WebsocketConfig;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct WebsocketToml {
    pub address: Option<String>,
    pub enable: Option<bool>,
    pub port: Option<u16>,
}

impl Default for WebsocketToml {
    fn default() -> Self {
        let config = WebsocketConfig::default();
        (&config).into()
    }
}

impl From<&WebsocketToml> for WebsocketConfig {
    fn from(toml: &WebsocketToml) -> Self {
        let mut config = WebsocketConfig::default();

        if let Some(enabled) = toml.enable {
            config.enabled = enabled;
        }
        if let Some(port) = toml.port {
            config.port = port;
        }
        if let Some(address) = &toml.address {
            config.address = address.clone();
        }
        config
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
