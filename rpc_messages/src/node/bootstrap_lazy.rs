use crate::{RpcBool, RpcBoolNumber};
use rsnano_core::BlockHash;
use serde::{Deserialize, Serialize};

impl From<BlockHash> for BootstrapLazyArgs {
    fn from(value: BlockHash) -> Self {
        Self::builder(value).build()
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct BootstrapLazyArgs {
    pub hash: BlockHash,
    pub force: Option<RpcBool>,
    pub id: Option<String>,
}

impl BootstrapLazyArgs {
    pub fn builder(hash: BlockHash) -> BootsrapLazyArgsBuilder {
        BootsrapLazyArgsBuilder {
            args: BootstrapLazyArgs {
                hash,
                force: None,
                id: None,
            },
        }
    }
}

pub struct BootsrapLazyArgsBuilder {
    args: BootstrapLazyArgs,
}

impl BootsrapLazyArgsBuilder {
    pub fn force(mut self) -> Self {
        self.args.force = Some(true.into());
        self
    }

    pub fn id(mut self, id: String) -> Self {
        self.args.id = Some(id);
        self
    }

    pub fn build(self) -> BootstrapLazyArgs {
        self.args
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct BootstrapLazyResponse {
    pub started: RpcBoolNumber,
    pub key_inserted: RpcBoolNumber,
}
