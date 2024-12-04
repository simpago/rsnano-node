use crate::{RpcCommand, RpcU64};
use rsnano_core::HashOrAccount;
use rsnano_core::{BlockHash, JsonBlock};
use serde::{Deserialize, Serialize};

impl RpcCommand {
    pub fn unchecked_keys(key: HashOrAccount, count: Option<u64>) -> Self {
        Self::UncheckedKeys(UncheckedKeysArgs {
            key,
            count: count.map(|i| i.into()),
        })
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct UncheckedKeysArgs {
    pub key: HashOrAccount,
    pub count: Option<RpcU64>,
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct UncheckedKeysResponse {
    pub unchecked: Vec<UncheckedKeyDto>,
}

impl UncheckedKeysResponse {
    pub fn new(unchecked: Vec<UncheckedKeyDto>) -> Self {
        Self { unchecked }
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct UncheckedKeyDto {
    pub key: BlockHash,
    pub hash: BlockHash,
    pub modified_timestamp: RpcU64,
    pub contents: JsonBlock,
}
