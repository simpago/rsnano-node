use anyhow::Result;
use rsnano_core::utils::TomlWriter;

pub struct OpenclConfig {
    pub enable: bool,
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
