use crate::config::OpenclConfig;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct OpenclToml {
    pub device: Option<u32>,
    pub enable: Option<bool>,
    pub platform: Option<u32>,
    pub threads: Option<u32>,
}

impl OpenclConfig {
    pub fn merge_toml(&mut self, toml: &OpenclToml) {
        if let Some(device) = toml.device {
            self.device = device;
        }
        if let Some(platform) = toml.platform {
            self.platform = platform;
        }
        if let Some(threads) = toml.threads {
            self.threads = threads;
        }
    }
}
