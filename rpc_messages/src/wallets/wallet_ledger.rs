use crate::{RpcBool, RpcCommand, RpcU64};
use rsnano_core::WalletId;
use rsnano_core::{Account, Amount, BlockHash};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

impl RpcCommand {
    pub fn wallet_ledger(args: WalletLedgerArgs) -> Self {
        Self::WalletLedger(args)
    }
}

impl From<WalletId> for WalletLedgerArgs {
    fn from(value: WalletId) -> Self {
        Self::builder(value).build()
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct WalletLedgerArgs {
    pub wallet: WalletId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub representative: Option<RpcBool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<RpcBool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receivable: Option<RpcBool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_since: Option<RpcU64>,
}

impl WalletLedgerArgs {
    pub fn builder(wallet: WalletId) -> WalletLedgerArgsBuilder {
        WalletLedgerArgsBuilder {
            args: WalletLedgerArgs {
                wallet,
                representative: None,
                weight: None,
                receivable: None,
                modified_since: None,
            },
        }
    }
}

pub struct WalletLedgerArgsBuilder {
    args: WalletLedgerArgs,
}

impl WalletLedgerArgsBuilder {
    pub fn representative(mut self) -> Self {
        self.args.representative = Some(true.into());
        self
    }

    pub fn receivable(mut self) -> Self {
        self.args.receivable = Some(true.into());
        self
    }

    pub fn weight(mut self) -> Self {
        self.args.weight = Some(true.into());
        self
    }

    pub fn modified_since(mut self, value: u64) -> Self {
        self.args.modified_since = Some(value.into());
        self
    }

    pub fn build(self) -> WalletLedgerArgs {
        self.args
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct WalletLedgerResponse {
    pub accounts: HashMap<Account, AccountInfo>,
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AccountInfo {
    pub frontier: BlockHash,
    pub open_block: BlockHash,
    pub representative_block: BlockHash,
    pub balance: Amount,
    pub modified_timestamp: RpcU64,
    pub block_count: RpcU64,
    pub representative: Option<Account>,
    pub weight: Option<Amount>,
    pub pending: Option<Amount>,
    pub receivable: Option<Amount>,
}
