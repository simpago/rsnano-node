use crate::{RpcU32, RpcU64, RpcUsize};
use rsnano_core::{Amount, BlockHash};
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ConfirmationHistoryArgs {
    pub hash: Option<BlockHash>,
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ConfirmationHistoryResponse {
    pub confirmations: Vec<ConfirmationEntry>,
    pub confirmation_stats: ConfirmationStats,
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ConfirmationEntry {
    pub hash: BlockHash,
    pub duration: RpcU64,
    pub time: RpcU64,
    pub tally: Amount,
    #[serde(rename = "final")]
    pub final_tally: Amount,
    pub blocks: RpcU32,
    pub voters: RpcU32,
    pub request_count: RpcU32,
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ConfirmationStats {
    pub count: RpcUsize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub average: Option<RpcU64>,
}
