use rsnano_core::{Account, Amount, BlockHash, BlockSubType, Signature, WorkNonce};
use serde::{Deserialize, Serialize};
use crate::RpcCommand;

impl RpcCommand {
    pub fn account_history(
        account: Account,
        count: u64,
        raw: Option<bool>,
        head: Option<BlockHash>,
        offset: Option<u64>,
        reverse: Option<bool>,
        account_filter: Option<Vec<Account>>,
    ) -> Self {
        Self::AccountHistory(AccountHistoryArgs::new(
            account,
            count,
            raw,
            head,
            offset,
            reverse,
            account_filter,
        ))
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AccountHistoryArgs {
    pub account: Account,
    pub count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head: Option<BlockHash>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reverse: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_filter: Option<Vec<Account>>,
}

impl AccountHistoryArgs {
    pub fn new(
        account: Account,
        count: u64,
        raw: Option<bool>,
        head: Option<BlockHash>,
        offset: Option<u64>,
        reverse: Option<bool>,
        account_filter: Option<Vec<Account>>,
    ) -> Self {
        Self {
            account,
            count,
            raw,
            head,
            offset,
            reverse,
            account_filter,
        }
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AccountHistoryDto {
    pub account: Account,
    pub history: Vec<HistoryEntry>,
    pub previous: Option<BlockHash>,
    pub next: Option<BlockHash>,
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct HistoryEntry {
    #[serde(rename = "type")]
    pub block_type: BlockSubType,
    pub account: Account,
    pub amount: Amount,
    pub local_timestamp: u64,
    pub height: u64,
    pub hash: BlockHash,
    pub confirmed: bool,
    pub work: Option<WorkNonce>,
    pub signature: Option<Signature>
}

