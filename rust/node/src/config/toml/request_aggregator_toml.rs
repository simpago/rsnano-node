use crate::consensus::RequestAggregatorConfig;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct RequestAggregatorToml {
    pub batch_size: Option<usize>,
    pub max_queue: Option<usize>,
    pub threads: Option<usize>,
}

impl Default for RequestAggregatorToml {
    fn default() -> Self {
        let config = RequestAggregatorConfig::default();
        (&config).into()
    }
}

impl From<&RequestAggregatorToml> for RequestAggregatorConfig {
    fn from(toml: &RequestAggregatorToml) -> Self {
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

impl From<&RequestAggregatorConfig> for RequestAggregatorToml {
    fn from(config: &RequestAggregatorConfig) -> Self {
        Self {
            threads: Some(config.threads),
            max_queue: Some(config.max_queue),
            batch_size: Some(config.batch_size),
        }
    }
}
