use crate::config::OpenclConfig;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct OpenclConfigToml {
    pub platform: u32,
    pub device: u32,
    pub threads: u32,
}
impl From<OpenclConfig> for OpenclConfigToml {
    fn from(config: OpenclConfig) -> Self {
        Self {
            platform: config.platform,
            device: config.device,
            threads: config.threads,
        }
    }
}
