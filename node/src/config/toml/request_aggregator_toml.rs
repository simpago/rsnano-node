use crate::consensus::RequestAggregatorConfig;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct RequestAggregatorToml {
    pub batch_size: Option<usize>,
    pub max_queue: Option<usize>,
    pub threads: Option<usize>,
}

impl RequestAggregatorConfig {
    pub fn merge_toml(&mut self, toml: &RequestAggregatorToml) {
        if let Some(threads) = toml.threads {
            self.threads = threads;
        }
        if let Some(max_queue) = toml.max_queue {
            self.max_queue = max_queue;
        }
        if let Some(batch_size) = toml.batch_size {
            self.batch_size = batch_size;
        }
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
