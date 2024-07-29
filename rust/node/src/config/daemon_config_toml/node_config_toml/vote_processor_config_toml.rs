use crate::consensus::VoteProcessorConfig;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct VoteProcessorConfigToml {
    pub max_pr_queue: Option<usize>,
    pub max_non_pr_queue: Option<usize>,
    pub pr_priority: Option<usize>,
    pub threads: Option<usize>,
    pub batch_size: Option<usize>,
    pub max_triggered: Option<usize>,
}

impl Default for VoteProcessorConfigToml {
    fn default() -> Self {
        let config = VoteProcessorConfig::default();
        Self {
            max_pr_queue: Some(config.max_non_pr_queue),
            max_non_pr_queue: Some(config.max_non_pr_queue),
            pr_priority: Some(config.pr_priority),
            threads: Some(config.threads),
            batch_size: Some(config.batch_size),
            max_triggered: Some(config.max_triggered),
        }
    }
}

impl From<&VoteProcessorConfigToml> for VoteProcessorConfig {
    fn from(toml: &VoteProcessorConfigToml) -> Self {
        let mut config = VoteProcessorConfig::default();

        if let Some(max_pr_queue) = toml.max_pr_queue {
            config.max_pr_queue = max_pr_queue;
        }
        if let Some(max_non_pr_queue) = toml.max_non_pr_queue {
            config.max_non_pr_queue = max_non_pr_queue;
        }
        if let Some(batch_size) = toml.pr_priority {
            config.batch_size = batch_size;
        }
        if let Some(threads) = toml.threads {
            config.batch_size = threads;
        }
        if let Some(batch_size) = toml.batch_size {
            config.batch_size = batch_size;
        }
        if let Some(max_triggered) = toml.max_triggered {
            config.max_triggered = max_triggered;
        }
        config
    }
}
