use crate::{config::TomlConfigOverride, transport::MessageProcessorConfig};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct MessageProcessorConfigToml {
    pub threads: Option<usize>,
    pub max_queue: Option<usize>,
}

impl From<MessageProcessorConfig> for MessageProcessorConfigToml {
    fn from(config: MessageProcessorConfig) -> Self {
        Self {
            threads: Some(config.threads),
            max_queue: Some(config.max_queue),
        }
    }
}

impl<'de> TomlConfigOverride<'de, MessageProcessorConfigToml> for MessageProcessorConfig {
    fn toml_config_override(&mut self, toml: &'de MessageProcessorConfigToml) {
        if let Some(threads) = toml.threads {
            self.threads = threads;
        }
        if let Some(max_queue) = toml.max_queue {
            self.max_queue = max_queue;
        }
    }
}
