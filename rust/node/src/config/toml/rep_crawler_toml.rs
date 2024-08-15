use std::time::Duration;

use crate::config::NodeConfig;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct RepCrawlerToml {
    pub query_timeout: Option<u64>,
}

impl Default for RepCrawlerToml {
    fn default() -> Self {
        let config = NodeConfig::default();
        (&config).into()
    }
}

impl From<&RepCrawlerToml> for NodeConfig {
    fn from(toml: &RepCrawlerToml) -> Self {
        let mut config = NodeConfig::default();

        if let Some(query_timeout) = toml.query_timeout {
            config.rep_crawler_query_timeout = Duration::from_millis(query_timeout);
        }
        config
    }
}

impl From<&NodeConfig> for RepCrawlerToml {
    fn from(config: &NodeConfig) -> Self {
        Self {
            query_timeout: Some(config.rep_crawler_query_timeout.as_millis() as u64),
        }
    }
}
