use anyhow::Result;
use rsnano_core::utils::TomlWriter;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct OpenclConfig {
    pub platform: u32,
    pub device: u32,
    pub threads: u32,
}

impl OpenclConfig {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn serialize_toml(&self, toml: &mut dyn TomlWriter) -> Result<()> {
        toml.put_u32("platform", self.platform, "OpenCL platform identifier")?;
        toml.put_u32("device", self.device, "OpenCL device identifier")?;
        toml.put_u32("threads", self.threads, "OpenCL thread count")?;
        Ok(())
    }

    pub fn deserialize_toml(&mut self, table: &toml::value::Table) -> Result<()> {
        if let Some(platform) = table
            .get("platform")
            .and_then(|v| v.as_integer().map(|v| v as u32))
        {
            self.platform = platform;
        }
        if let Some(device) = table
            .get("device")
            .and_then(|v| v.as_integer().map(|v| v as u32))
        {
            self.device = device;
        }
        if let Some(threads) = table
            .get("threads")
            .and_then(|v| v.as_integer().map(|v| v as u32))
        {
            self.threads = threads;
        }
        Ok(())
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
