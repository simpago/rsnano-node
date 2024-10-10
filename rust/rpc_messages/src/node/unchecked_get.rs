use crate::{BlockHashRpcMessage, RpcCommand};
use rsnano_core::{BlockHash, JsonBlock};
use serde::{Deserialize, Serialize};

impl RpcCommand {
    pub fn unchecked_get(hash: BlockHash) -> Self {
        Self::UncheckedGet(BlockHashRpcMessage::new("hash".to_string(), hash))
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]

pub struct UncheckedGetDto {
    pub modified_timestamp: u64,
    pub contents: JsonBlock,
}

impl UncheckedGetDto {
    pub fn new(modified_timestamp: u64, contents: JsonBlock) -> Self {
        Self {
            modified_timestamp,
            contents,
        }
    }
}
