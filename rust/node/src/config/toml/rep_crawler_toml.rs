use crate::config::NodeConfig;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Deserialize, Serialize)]
pub struct RepCrawlerToml {
    pub query_timeout: Option<u64>,
}

impl NodeConfig {
    pub fn merge_rep_crawler_toml(&mut self, toml: &RepCrawlerToml) {
        if let Some(query_timeout) = toml.query_timeout {
            self.rep_crawler_query_timeout = Duration::from_millis(query_timeout);
        }
    }
}

impl From<&NodeConfig> for RepCrawlerToml {
    fn from(config: &NodeConfig) -> Self {
        Self {
            query_timeout: Some(config.rep_crawler_query_timeout.as_millis() as u64),
        }
    }
}
