use anyhow::Result;
use rsnano_core::utils::TomlWriter;
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

    pub fn serialize_toml(&self, toml: &mut dyn TomlWriter) -> Result<()> {
        toml.put_u32("platform", self.platform, "OpenCL platform identifier")?;
        toml.put_u32("device", self.device, "OpenCL device identifier")?;
        toml.put_u32("threads", self.threads, "OpenCL thread count")?;
        Ok(())
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
