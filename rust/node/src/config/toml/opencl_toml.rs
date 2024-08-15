use crate::config::{DaemonConfig, OpenclConfig};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct OpenclToml {
    pub device: Option<u32>,
    pub enable: Option<bool>,
    pub platform: Option<u32>,
    pub threads: Option<u32>,
}

impl OpenclToml {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Default for OpenclToml {
    fn default() -> Self {
        let config = DaemonConfig::default();
        (&config).into()
    }
}

impl From<&OpenclToml> for OpenclConfig {
    fn from(toml: &OpenclToml) -> Self {
        let mut config = OpenclConfig::default();
        if let Some(device) = toml.device {
            config.device = device;
        }
        if let Some(platform) = toml.platform {
            config.platform = platform;
        }
        if let Some(threads) = toml.threads {
            config.threads = threads;
        }

        config
    }
}
