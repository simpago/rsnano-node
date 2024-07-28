use std::cmp::{max, min};

use rsnano_core::utils::{get_cpu_count, TomlWriter};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct MessageProcessorConfig {
    pub threads: usize,
    pub max_queue: usize,
}

impl Default for MessageProcessorConfig {
    fn default() -> Self {
        let parallelism = get_cpu_count();

        Self {
            threads: min(2, max(parallelism / 4, 1)),
            max_queue: 64,
        }
    }
}

impl MessageProcessorConfig {
    pub fn new(parallelism: usize) -> Self {
        Self {
            threads: min(2, max(parallelism / 4, 1)),
            max_queue: 64,
        }
    }

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

#[derive(Deserialize, Serialize)]
pub struct MessageProcessorConfigToml {
    pub threads: Option<usize>,
    pub max_queue: Option<usize>,
}

impl From<&MessageProcessorConfig> for MessageProcessorConfigToml {
    fn from(config: &MessageProcessorConfig) -> Self {
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
