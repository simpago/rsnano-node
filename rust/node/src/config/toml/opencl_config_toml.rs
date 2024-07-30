use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct OpenclConfigToml {
    pub enable: Option<bool>,
    pub platform: Option<u32>,
    pub device: Option<u32>,
    pub threads: Option<u32>,
}

impl OpenclConfigToml {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Default for OpenclConfigToml {
    fn default() -> Self {
        Self {
            enable: Some(false),
            platform: Some(0),
            device: Some(0),
            threads: Some(1024 * 1024),
        }
    }
}
