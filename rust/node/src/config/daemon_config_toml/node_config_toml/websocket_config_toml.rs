use anyhow::Result;
use rsnano_core::utils::TomlWriter;
use serde::{Deserialize, Serialize};

use crate::websocket::WebsocketConfig;

#[derive(Deserialize, Serialize)]
pub struct WebsocketConfigToml {
    pub enabled: Option<bool>,
    pub port: Option<u16>,
    pub address: Option<String>,
}

impl WebsocketConfigToml {
    pub fn serialize_toml(&self, toml: &mut dyn TomlWriter) -> Result<()> {
        toml.put_bool(
            "enable",
            self.enabled,
            "Enable or disable WebSocket server.\ntype:bool",
        )?;
        toml.put_str(
            "address",
            &self.address,
            "WebSocket server bind address.\ntype:string,ip",
        )?;
        toml.put_u16(
            "port",
            self.port,
            "WebSocket server listening port.\ntype:uint16",
        )?;
        Ok(())
    }
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
