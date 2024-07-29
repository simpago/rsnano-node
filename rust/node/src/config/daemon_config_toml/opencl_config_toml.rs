use anyhow::Result;
use rsnano_core::utils::TomlWriter;
use serde::{Deserialize, Serialize};

pub struct OpenclConfig {
    pub enable: bool,
    pub platform: u32,
    pub device: u32,
    pub threads: u32,
}

impl OpenclConfigToml {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn serialize_toml(&self, toml: &mut dyn TomlWriter) -> Result<()> {
        toml.put_u32("platform", self.platform, "OpenCL platform identifier")?;
        toml.put_u32("device", self.device, "OpenCL device identifier")?;
        toml.put_u32("threads", self.threads, "OpenCL thread count")?;
        Ok(())
    }
}

impl Default for OpenclConfig {
    fn default() -> Self {
        Self {
            enable: false,
            platform: 0,
            device: 0,
            threads: 1024 * 1024,
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct OpenclConfigToml {
    pub platform: Option<u32>,
    pub device: Option<u32>,
    pub threads: Option<u32>,
}
impl From<&OpenclConfig> for OpenclConfigToml {
    fn from(config: &OpenclConfig) -> Self {
        Self {
            platform: Some(config.platform),
            device: Some(config.device),
            threads: Some(config.threads),
        }
    }
}
