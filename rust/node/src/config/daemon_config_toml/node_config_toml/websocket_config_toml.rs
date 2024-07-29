use crate::websocket::WebsocketConfig;
use anyhow::Result;
use rsnano_core::utils::TomlWriter;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct WebsocketConfigToml {
    pub enabled: Option<bool>,
    pub port: Option<u16>,
    pub address: Option<String>,
}

impl Default for WebsocketConfigToml {
    fn default() -> Self {
        let config = WebsocketConfig::default();
        Self {
            enabled: Some(config.enabled),
            port: Some(config.port),
            address: Some(config.address),
        }
    }
}

impl From<&WebsocketConfigToml> for WebsocketConfig {
    fn from(toml: &WebsocketConfigToml) -> Self {
        let mut config = WebsocketConfig::default();

        if let Some(enabled) = toml.enabled {
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
