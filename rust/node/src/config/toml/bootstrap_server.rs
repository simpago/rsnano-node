use crate::config::BootstrapServerConfig;

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
