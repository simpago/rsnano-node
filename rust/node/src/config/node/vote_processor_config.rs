use std::cmp::{max, min};

use rsnano_core::utils::{get_cpu_count, TomlWriter};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct VoteProcessorConfig {
    pub max_pr_queue: usize,
    pub max_non_pr_queue: usize,
    pub pr_priority: usize,
    pub threads: usize,
    pub batch_size: usize,
    pub max_triggered: usize,
}

impl Default for VoteProcessorConfig {
    fn default() -> Self {
        let parallelism = get_cpu_count();

        Self {
            max_pr_queue: 256,
            max_non_pr_queue: 32,
            pr_priority: 3,
            threads: max(1, min(4, parallelism / 2)),
            batch_size: 1024,
            max_triggered: 16384,
        }
    }
}

impl VoteProcessorConfig {
    pub fn new(parallelism: usize) -> Self {
        Self {
            max_pr_queue: 256,
            max_non_pr_queue: 32,
            pr_priority: 3,
            threads: max(1, min(4, parallelism / 2)),
            batch_size: 1024,
            max_triggered: 16384,
        }
    }

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

#[derive(Deserialize, Serialize)]
pub struct VoteProcessorConfigToml {
    pub max_pr_queue: Option<usize>,
    pub max_non_pr_queue: Option<usize>,
    pub pr_priority: Option<usize>,
    pub threads: Option<usize>,
    pub batch_size: Option<usize>,
    pub max_triggered: Option<usize>,
}

impl From<&VoteProcessorConfig> for VoteProcessorConfigToml {
    fn from(config: &VoteProcessorConfig) -> Self {
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
