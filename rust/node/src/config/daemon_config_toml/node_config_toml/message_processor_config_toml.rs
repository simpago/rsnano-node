use crate::transport::MessageProcessorConfig;
use rsnano_core::utils::TomlWriter;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct MessageProcessorConfigToml {
    pub threads: Option<usize>,
    pub max_queue: Option<usize>,
}

impl MessageProcessorConfigToml {
    pub fn serialize_toml(&self, toml: &mut dyn TomlWriter) -> anyhow::Result<()> {
        toml.put_usize(
            "threads",
            self.threads,
            "Number of threads to use for message processing. \ntype:uint64",
        )?;

        toml.put_usize(
            "max_queue",
            self.max_queue,
            "Maximum number of messages per peer to queue for processing. \ntype:uint64",
        )
    }
}

impl Default for MessageProcessorConfigToml {
    fn default() -> Self {
        let config = MessageProcessorConfig::default();
        Self {
            threads: Some(config.threads),
            max_queue: Some(config.max_queue),
        }
    }
}

impl From<&MessageProcessorConfigToml> for MessageProcessorConfig {
    fn from(toml: &MessageProcessorConfigToml) -> Self {
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
