use std::time::Duration;

use rsnano_core::utils::TomlWriter;
use serde::{Deserialize, Serialize};

use crate::config::Miliseconds;

#[derive(Clone)]
pub struct AccountSetsConfig {
    pub consideration_count: usize,
    pub priorities_max: usize,
    pub blocking_max: usize,
    pub cooldown: Duration,
}

impl Default for AccountSetsConfig {
    fn default() -> Self {
        Self {
            consideration_count: 4,
            priorities_max: 256 * 1024,
            blocking_max: 256 * 1024,
            cooldown: Duration::from_secs(3),
        }
    }
}

#[derive(Deserialize, Serialize, Default)]
pub struct AccountSetsToml {
    pub consideration_count: Option<usize>,
    pub priorities_max: Option<usize>,
    pub blocking_max: Option<usize>,
    pub cooldown: Option<Miliseconds>,
}

impl From<AccountSetsConfig> for AccountSetsToml {
    fn from(value: AccountSetsConfig) -> Self {
        Self {
            consideration_count: Some(value.consideration_count),
            priorities_max: Some(value.priorities_max),
            blocking_max: Some(value.blocking_max),
            cooldown: Some(Miliseconds(value.cooldown.as_millis())),
        }
    }
}

impl AccountSetsConfig {
    pub(crate) fn serialize_toml(&self, toml: &mut dyn TomlWriter) -> anyhow::Result<()> {
        toml.put_usize ("consideration_count", self.consideration_count, "Limit the number of account candidates to consider and also the number of iterations.\ntype:uint64")?;
        toml.put_usize(
            "priorities_max",
            self.priorities_max,
            "Cutoff size limit for the priority list.\ntype:uint64",
        )?;
        toml.put_usize(
            "blocking_max",
            self.blocking_max,
            "Cutoff size limit for the blocked accounts from the priority list.\ntype:uint64",
        )?;
        toml.put_u64(
            "cooldown",
            self.cooldown.as_millis() as u64,
            "Waiting time for an account to become available.\ntype:milliseconds",
        )
    }
}
