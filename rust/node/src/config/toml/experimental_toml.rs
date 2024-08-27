use std::str::FromStr;

use crate::config::{NodeConfig, Peer};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct ExperimentalToml {
    pub max_pruning_age: Option<u64>,
    pub max_pruning_depth: Option<u64>,
    pub secondary_work_peers: Option<Vec<String>>,
}

impl NodeConfig {
    pub fn merge_experimental_toml(&mut self, toml: &ExperimentalToml) {
        if let Some(max_pruning_age) = toml.max_pruning_age {
            self.max_pruning_age_s = max_pruning_age as i64;
        }
        if let Some(max_pruning_depth) = toml.max_pruning_depth {
            self.max_pruning_depth = max_pruning_depth;
        }
        if let Some(secondary_work_peers) = &toml.secondary_work_peers {
            self.secondary_work_peers = secondary_work_peers
                .iter()
                .map(|string| Peer::from_str(&string).expect("Invalid secondary work peer"))
                .collect();
        }
    }
}

impl From<&NodeConfig> for ExperimentalToml {
    fn from(config: &NodeConfig) -> Self {
        Self {
            secondary_work_peers: Some(
                config
                    .secondary_work_peers
                    .iter()
                    .map(|peer| peer.to_string())
                    .collect(),
            ),
            max_pruning_age: Some(config.max_pruning_age_s as u64),
            max_pruning_depth: Some(config.max_pruning_depth),
        }
    }
}
