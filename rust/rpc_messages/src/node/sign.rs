use rsnano_core::{Account, JsonBlock, RawKey, Signature, WalletId};
use serde::{Deserialize, Serialize};
use crate::RpcCommand;

impl RpcCommand {
    pub fn sign(key: Option<RawKey>, wallet: Option<WalletId>, account: Option<Account>, block: JsonBlock) -> Self {
        Self::Sign(SignArgs::new(key, wallet, account, block))
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]

pub struct SignArgs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<RawKey>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet: Option<WalletId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<Account>,
    pub block: JsonBlock,
}

impl SignArgs {
    pub fn new(key: Option<RawKey>, wallet: Option<WalletId>, account: Option<Account>, block: JsonBlock) -> Self {
        Self { key, wallet, account, block }
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]

pub struct SignDto {
    pub signature: Signature,
    pub block: JsonBlock,
}

impl SignDto {
    pub fn new(signature: Signature, block: JsonBlock) -> Self {
        Self { signature, block }
    }
}