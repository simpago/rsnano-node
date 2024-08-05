use crate::transport::MessageProcessorConfig;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct MessageProcessorToml {
    pub max_queue: Option<usize>,
    pub threads: Option<usize>,
}

impl Default for MessageProcessorToml {
    fn default() -> Self {
        let config = MessageProcessorConfig::default();
        (&config).into()
    }
}

impl From<&MessageProcessorToml> for MessageProcessorConfig {
    fn from(toml: &MessageProcessorToml) -> Self {
        let mut config = MessageProcessorConfig::default();

        if let Some(threads) = toml.threads {
            config.threads = threads;
        }
        if let Some(max_queue) = toml.max_queue {
            config.max_queue = max_queue;
        }
        config
    }
}

impl From<&MessageProcessorConfig> for MessageProcessorToml {
    fn from(config: &MessageProcessorConfig) -> Self {
        Self {
            threads: Some(config.threads),
            max_queue: Some(config.max_queue),
        }
    }
}
