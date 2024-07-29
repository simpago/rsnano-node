use crate::consensus::RequestAggregatorConfig;
use rsnano_core::utils::TomlWriter;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct RequestAggregatorConfigToml {
    pub threads: Option<usize>,
    pub max_queue: Option<usize>,
    pub batch_size: Option<usize>,
}

impl RequestAggregatorConfigToml {
    pub fn serialize_toml(&self, toml: &mut dyn TomlWriter) -> anyhow::Result<()> {
        toml.put_usize(
            "max_queue",
            self.max_queue,
            "Maximum number of queued requests per peer. \ntype:uint64",
        )?;
        toml.put_usize(
            "threads",
            self.threads,
            "Number of threads for request processing. \ntype:uint64",
        )?;
        toml.put_usize(
            "batch_size",
            self.batch_size,
            "Number of requests to process in a single batch. \ntype:uint64",
        )
    }
}

impl Default for RequestAggregatorConfigToml {
    fn default() -> Self {
        let config = RequestAggregatorConfig::default();
        Self {
            threads: Some(config.threads),
            max_queue: Some(config.max_queue),
            batch_size: Some(config.batch_size),
        }
    }
}

impl From<&RequestAggregatorConfigToml> for RequestAggregatorConfig {
    fn from(toml: &RequestAggregatorConfigToml) -> Self {
        let mut config = RequestAggregatorConfig::default();

        if let Some(threads) = toml.threads {
            config.threads = threads;
        }
        if let Some(max_queue) = toml.max_queue {
            config.max_queue = max_queue;
        }
        if let Some(batch_size) = toml.batch_size {
            config.batch_size = batch_size;
        }
        config
    }
}
