use rsnano_core::BlockHash;
use serde::{Deserialize, Serialize};
use crate::RpcCommand;

impl RpcCommand {
    pub fn bootstrap_lazy(hash: BlockHash, force: Option<bool>, id: Option<String>) -> Self {
        Self::BoostrapLazy(BootsrapLazyArgs::new(hash, force, id))
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]

pub struct BootsrapLazyArgs {
    pub hash: BlockHash,
    pub force: Option<bool>,
    pub id: Option<String>,
}

impl BootsrapLazyArgs {
    pub fn new(hash: BlockHash, force: Option<bool>, id: Option<String>) -> Self {
        Self {
            hash,
            force,
            id,
        }
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct BootstrapLazyDto {
    pub started: bool,
    pub key_inserted: bool,
}

impl BootstrapLazyDto {
    pub fn new(started: bool, key_inserted: bool) -> Self {
        Self {
            started,
            key_inserted,
        }
    }
}