use super::BlockInfoResponse;
use crate::{RpcBool, RpcCommand};
use rsnano_core::BlockHash;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

impl RpcCommand {
    pub fn blocks_info(args: impl Into<BlocksInfoArgs>) -> Self {
        Self::BlocksInfo(args.into())
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct BlocksInfoResponse {
    pub blocks: HashMap<BlockHash, BlockInfoResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocks_not_found: Option<Vec<BlockHash>>,
}

impl BlocksInfoResponse {
    pub fn new(blocks: HashMap<BlockHash, BlockInfoResponse>) -> Self {
        Self {
            blocks,
            blocks_not_found: None,
        }
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct BlocksInfoArgs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receivable: Option<RpcBool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receive_hash: Option<RpcBool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<RpcBool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_not_found: Option<RpcBool>,
    pub hashes: Vec<BlockHash>,
}

impl From<Vec<BlockHash>> for BlocksInfoArgs {
    fn from(value: Vec<BlockHash>) -> Self {
        Self {
            receivable: None,
            receive_hash: None,
            source: None,
            include_not_found: None,
            hashes: value,
        }
    }
}
