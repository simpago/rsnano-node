use crate::RpcCommand;
use rsnano_core::{Account, JsonBlock, RawKey, Signature, WalletId};
use serde::{Deserialize, Serialize};

impl RpcCommand {
    pub fn sign(args: SignArgs) -> Self {
        Self::Sign(args)
    }
}

impl From<JsonBlock> for SignArgs {
    fn from(value: JsonBlock) -> Self {
        Self::builder(value).build()
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct SignArgs {
    pub block: JsonBlock,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<RawKey>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet: Option<WalletId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<Account>,
}

impl SignArgs {
    pub fn builder(block: JsonBlock) -> SignArgsBuilder {
        SignArgsBuilder {
            args: SignArgs {
                block,
                key: None,
                wallet: None,
                account: None,
            },
        }
    }
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
pub struct SignDto {
    pub signature: Signature,
    pub block: JsonBlock,
}

impl SignDto {
    pub fn new(signature: Signature, block: JsonBlock) -> Self {
        Self { signature, block }
    }
}
