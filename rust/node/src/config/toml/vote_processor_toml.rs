use crate::consensus::VoteProcessorConfig;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct VoteProcessorToml {
    pub batch_size: Option<usize>,
    pub max_non_pr_queue: Option<usize>,
    pub max_pr_queue: Option<usize>,
    pub pr_priority: Option<usize>,
    pub threads: Option<usize>,
    //pub max_triggered: Option<usize>,
}

impl VoteProcessorConfig {
    pub fn merge_toml(&mut self, toml: &VoteProcessorToml) {
        if let Some(max_pr_queue) = toml.max_pr_queue {
            self.max_pr_queue = max_pr_queue;
        }
        if let Some(max_non_pr_queue) = toml.max_non_pr_queue {
            self.max_non_pr_queue = max_non_pr_queue;
        }
        if let Some(pr_priority) = toml.pr_priority {
            self.pr_priority = pr_priority;
        }
        if let Some(threads) = toml.threads {
            self.threads = threads;
        }
        if let Some(batch_size) = toml.batch_size {
            self.batch_size = batch_size;
        }
        //if let Some(max_triggered) = toml.max_triggered {
        //self.max_triggered = max_triggered;
        //}
    }
}

impl From<&VoteProcessorConfig> for VoteProcessorToml {
    fn from(config: &VoteProcessorConfig) -> Self {
        Self {
            max_pr_queue: Some(config.max_pr_queue),
            max_non_pr_queue: Some(config.max_non_pr_queue),
            pr_priority: Some(config.pr_priority),
            threads: Some(config.threads),
            batch_size: Some(config.batch_size),
            //max_triggered: Some(config.max_triggered),
        }
    }
}
