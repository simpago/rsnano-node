use indexmap::IndexMap;
use rsnano_core::QualifiedRoot;
use rsnano_core::{Account, Amount, BlockHash, JsonBlock};
use serde::{Deserialize, Serialize};

use crate::{RpcBool, RpcU32, RpcUsize};

impl From<QualifiedRoot> for ConfirmationInfoArgs {
    fn from(value: QualifiedRoot) -> Self {
        Self::build(value).finish()
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ConfirmationInfoArgs {
    pub root: QualifiedRoot,
    pub contents: Option<RpcBool>,
    pub representatives: Option<RpcBool>,
}

impl ConfirmationInfoArgs {
    pub fn build(root: QualifiedRoot) -> ConfirmationInfoArgsBuilder {
        ConfirmationInfoArgsBuilder {
            args: ConfirmationInfoArgs {
                root,
                contents: None,
                representatives: None,
            },
        }
    }
}

pub struct ConfirmationInfoArgsBuilder {
    args: ConfirmationInfoArgs,
}

impl ConfirmationInfoArgsBuilder {
    pub fn without_contents(mut self) -> Self {
        self.args.contents = Some(false.into());
        self
    }

    pub fn include_representatives(mut self) -> Self {
        self.args.representatives = Some(true.into());
        self
    }

    pub fn finish(self) -> ConfirmationInfoArgs {
        self.args
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ConfirmationInfoDto {
    pub announcements: RpcU32,
    pub voters: RpcUsize,
    pub last_winner: BlockHash,
    pub total_tally: Amount,
    pub final_tally: Amount,
    pub blocks: IndexMap<BlockHash, ConfirmationBlockInfoDto>,
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ConfirmationBlockInfoDto {
    pub tally: Amount,
    pub contents: Option<JsonBlock>,
    pub representatives: Option<IndexMap<Account, Amount>>,
}
