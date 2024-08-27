use crate::transport::MessageProcessorConfig;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct MessageProcessorToml {
    pub max_queue: Option<usize>,
    pub threads: Option<usize>,
}

impl MessageProcessorConfig {
    pub fn merge_toml(&mut self, toml: &MessageProcessorToml) {
        if let Some(threads) = toml.threads {
            self.threads = threads;
        }
        if let Some(max_queue) = toml.max_queue {
            self.max_queue = max_queue;
        }
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
