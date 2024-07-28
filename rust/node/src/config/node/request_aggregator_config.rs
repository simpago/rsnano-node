use crate::{config::TomlConfigOverride, consensus::RequestAggregatorConfig};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct RequestAggregatorConfigToml {
    pub threads: Option<usize>,
    pub max_queue: Option<usize>,
    pub batch_size: Option<usize>,
}

impl From<RequestAggregatorConfig> for RequestAggregatorConfigToml {
    fn from(config: RequestAggregatorConfig) -> Self {
        Self {
            threads: Some(config.threads),
            max_queue: Some(config.max_queue),
            batch_size: Some(config.batch_size),
        }
    }
}

impl<'de> TomlConfigOverride<'de, RequestAggregatorConfigToml> for RequestAggregatorConfig {
    fn toml_config_override(&mut self, toml: &'de RequestAggregatorConfigToml) {
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
