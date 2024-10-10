use crate::RpcCommand;
use rsnano_core::{Account, Amount, BlockHash, WalletId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

impl RpcCommand {
    pub fn wallet_ledger(
        wallet: WalletId,
        representative: Option<bool>,
        weight: Option<bool>,
        receivable: Option<bool>,
        modified_since: Option<u64>,
    ) -> Self {
        Self::WalletLedger(WalletLedgerArgs::new(
            wallet,
            representative,
            weight,
            receivable,
            modified_since,
        ))
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]

pub struct WalletLedgerArgs {
    pub wallet: WalletId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub representative: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receivable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_since: Option<u64>,
}

impl WalletLedgerArgs {
    pub fn new(
        wallet: WalletId,
        representative: Option<bool>,
        weight: Option<bool>,
        receivable: Option<bool>,
        modified_since: Option<u64>,
    ) -> Self {
        Self {
            wallet,
            representative,
            weight,
            receivable,
            modified_since,
        }
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct WalletLedgerDto {
    pub accounts: HashMap<Account, AccountInfo>,
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AccountInfo {
    pub frontier: BlockHash,
    pub open_block: BlockHash,
    pub representative_block: BlockHash,
    pub balance: Amount,
    pub modified_timestamp: u64,
    pub block_count: u64,
    pub representative: Option<Account>,
    pub weight: Option<Amount>,
    pub pending: Option<Amount>,
    pub receivable: Option<Amount>,
}

impl AccountInfo {
    pub fn new(
        frontier: BlockHash,
        open_block: BlockHash,
        representative_block: BlockHash,
        balance: Amount,
        modified_timestamp: u64,
        block_count: u64,
        representative: Option<Account>,
        weight: Option<Amount>,
        pending: Option<Amount>,
        receivable: Option<Amount>,
    ) -> Self {
        Self {
            frontier,
            open_block,
            representative_block,
            balance,
            modified_timestamp,
            block_count,
            representative,
            weight,
            pending,
            receivable,
        }
    }
}
