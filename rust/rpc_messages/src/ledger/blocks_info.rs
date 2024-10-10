use std::collections::HashMap;

use crate::{BlockInfoDto, BlocksHashesRpcMessage, RpcCommand};
use rsnano_core::BlockHash;
use serde::{Deserialize, Serialize};

impl RpcCommand {
    pub fn blocks_info(blocks: Vec<BlockHash>) -> Self {
        Self::BlocksInfo(BlocksHashesRpcMessage::new("hashes".to_string(), blocks))
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct BlocksInfoDto {
    blocks: HashMap<BlockHash, BlockInfoDto>,
}

impl BlocksInfoDto {
    pub fn new(blocks: HashMap<BlockHash, BlockInfoDto>) -> Self {
        Self { blocks }
    }
}
