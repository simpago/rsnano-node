use crate::config::NodeConfig;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct HttpcallbackToml {
    pub address: Option<String>,
    pub port: Option<u16>,
    pub target: Option<String>,
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

impl NodeConfig {
    pub fn merge_http_callback_toml(&mut self, toml: &HttpcallbackToml) {
        if let Some(address) = &toml.address {
            self.callback_address = address.clone();
        }
        if let Some(port) = &toml.port {
            self.callback_port = port.clone();
        }
        if let Some(target) = &toml.target {
            self.callback_target = target.clone();
        }
    }
}
