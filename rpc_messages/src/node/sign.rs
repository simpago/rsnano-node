use crate::RpcCommand;
use rsnano_core::{Account, BlockHash, JsonBlock, RawKey, Signature, WalletId};
use serde::{Deserialize, Serialize};

impl RpcCommand {
    pub fn sign(args: SignArgs) -> Self {
        Self::Sign(args)
    }
}

impl From<JsonBlock> for SignArgs {
    fn from(value: JsonBlock) -> Self {
        Self {
            block: Some(value),
            key: None,
            wallet: None,
            account: None,
            hash: None,
        }
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct SignArgs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block: Option<JsonBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<RawKey>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet: Option<WalletId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<Account>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<BlockHash>,
}

pub struct SignArgsBuilder {
    args: SignArgs,
}

impl SignArgsBuilder {
    pub fn key(mut self, key: RawKey) -> Self {
        self.args.key = Some(key);
        self
    }

    pub fn wallet(mut self, wallet: WalletId) -> Self {
        self.args.wallet = Some(wallet);
        self
    }

    pub fn account(mut self, account: Account) -> Self {
        self.args.account = Some(account);
        self
    }

    pub fn build(self) -> SignArgs {
        self.args
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct SignResponse {
    pub signature: Signature,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block: Option<JsonBlock>,
}
