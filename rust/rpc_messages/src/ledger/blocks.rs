use crate::{BlocksHashesRpcMessage, RpcCommand};
use rsnano_core::{BlockHash, JsonBlock};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

impl RpcCommand {
    pub fn blocks(blocks: Vec<BlockHash>) -> Self {
        Self::Blocks(BlocksHashesRpcMessage::new("hashes".to_string(), blocks))
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct BlocksDto {
    pub blocks: HashMap<BlockHash, JsonBlock>,
}

impl BlocksDto {
    pub fn new(blocks: HashMap<BlockHash, JsonBlock>) -> Self {
        Self { blocks }
    }
}
