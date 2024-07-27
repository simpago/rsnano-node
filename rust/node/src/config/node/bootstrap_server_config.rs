use rsnano_core::utils::TomlWriter;

#[derive(Clone, Debug)]
pub struct BootstrapServerConfig {
    pub max_queue: usize,
    pub threads: usize,
    pub batch_size: usize,
}

impl Default for BootstrapServerConfig {
    fn default() -> Self {
        Self {
            max_queue: 16,
            threads: 1,
            batch_size: 64,
        }
    }
}

impl BootstrapServerConfig {
    pub fn serialize_toml(&self, toml: &mut dyn TomlWriter) -> anyhow::Result<()> {
        toml.put_usize(
            "max_queue",
            self.max_queue,
            "Maximum number of queued requests per peer. \ntype:uint64",
        )?;
        toml.put_usize(
            "threads",
            self.threads,
            "Number of threads to process requests. \ntype:uint64",
        )?;
        toml.put_usize(
            "batch_size",
            self.batch_size,
            "Maximum number of requests to process in a single batch. \ntype:uint64",
        )
    }
}

#[derive(Clone, Debug)]
pub struct BootstrapServerConfigToml {
    pub max_queue: usize,
    pub threads: usize,
    pub batch_size: usize,
}

impl From<BootstrapServerConfig> for BootstrapServerConfigToml {
    fn from(config: BootstrapServerConfig) -> Self {
        Self {
            max_queue: config.max_queue,
            threads: config.threads,
            batch_size: config.batch_size,
        }
    }
}
