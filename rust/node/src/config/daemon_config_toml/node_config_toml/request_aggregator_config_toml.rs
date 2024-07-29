use crate::consensus::RequestAggregatorConfig;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct RequestAggregatorConfigToml {
    pub threads: Option<usize>,
    pub max_queue: Option<usize>,
    pub batch_size: Option<usize>,
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
