use crate::config::NodeConfig;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct HttpcallbackToml {
    pub address: Option<String>,
    pub port: Option<u16>,
    pub target: Option<String>,
}

impl Default for HttpcallbackToml {
    fn default() -> Self {
        let config = NodeConfig::default();
        (&config).into()
    }
}

impl From<&NodeConfig> for HttpcallbackToml {
    fn from(config: &NodeConfig) -> Self {
        Self {
            address: Some(config.callback_address.clone()),
            port: Some(config.callback_port.clone()),
            target: Some(config.callback_target.clone()),
        }
    }
}

impl From<&HttpcallbackToml> for NodeConfig {
    fn from(toml: &HttpcallbackToml) -> Self {
        let mut config = NodeConfig::default();
        if let Some(address) = &toml.address {
            config.callback_address = address.clone();
        }
        if let Some(port) = &toml.port {
            config.callback_port = port.clone();
        }
        if let Some(target) = &toml.target {
            config.callback_target = target.clone();
        }
        config
    }
}
