use serde::{Deserialize, Serialize};

pub struct OpenclConfig {
    pub platform: u32,
    pub device: u32,
    pub threads: u32,
}

impl OpenclConfig {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Default for OpenclConfig {
    fn default() -> Self {
        Self {
            platform: 0,
            device: 0,
            threads: 1024 * 1024,
        }
    }
}

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
