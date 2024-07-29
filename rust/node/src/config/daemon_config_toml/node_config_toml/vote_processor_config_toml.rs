use rsnano_core::utils::TomlWriter;
use serde::{Deserialize, Serialize};

use crate::consensus::VoteProcessorConfig;

#[derive(Deserialize, Serialize)]
pub struct VoteProcessorConfigToml {
    pub max_pr_queue: Option<usize>,
    pub max_non_pr_queue: Option<usize>,
    pub pr_priority: Option<usize>,
    pub threads: Option<usize>,
    pub batch_size: Option<usize>,
    pub max_triggered: Option<usize>,
}

impl VoteProcessorConfigToml {
    pub fn serialize_toml(&self, toml: &mut dyn TomlWriter) -> anyhow::Result<()> {
        toml.put_usize(
            "max_pr_queue",
            self.max_pr_queue,
            "Maximum number of votes to queue from principal representatives. \ntype:uint64",
        )?;

        toml.put_usize(
            "max_non_pr_queue",
            self.max_non_pr_queue,
            "Maximum number of votes to queue from non-principal representatives. \ntype:uint64",
        )?;

        toml.put_usize(
                "pr_priority",
                self.pr_priority,
                "Priority for votes from principal representatives. Higher priority gets processed more frequently. Non-principal representatives have a baseline priority of 1. \ntype:uint64",
            )?;

        toml.put_usize(
            "threads",
            self.threads,
            "Number of threads to use for processing votes. \ntype:uint64",
        )?;
        toml.put_usize(
            "batch_size",
            self.batch_size,
            "Maximum number of votes to process in a single batch. \ntype:uint64",
        )
    }
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
