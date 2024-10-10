use crate::RpcCommand;
use rsnano_core::{Account, Amount, BlockHash, JsonBlock, QualifiedRoot};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

impl RpcCommand {
    pub fn confirmation_info(
        root: QualifiedRoot,
        contents: Option<bool>,
        representatives: Option<bool>,
    ) -> Self {
        Self::ConfirmationInfo(ConfirmationInfoArgs::new(root, contents, representatives))
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ConfirmationInfoArgs {
    pub root: QualifiedRoot,
    pub contents: Option<bool>,
    pub representatives: Option<bool>,
}

impl ConfirmationInfoArgs {
    pub fn new(root: QualifiedRoot, contents: Option<bool>, representatives: Option<bool>) -> Self {
        Self {
            root,
            contents,
            representatives,
        }
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ConfirmationInfoDto {
    pub announcements: u32,
    pub voters: usize, // New field
    pub last_winner: BlockHash,
    pub total_tally: Amount,
    pub final_tally: Amount, // New field
    pub blocks: HashMap<BlockHash, ConfirmationBlockInfoDto>,
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ConfirmationBlockInfoDto {
    pub tally: Amount,
    pub contents: Option<JsonBlock>,
    pub representatives: Option<HashMap<Account, Amount>>,
}

impl ConfirmationInfoDto {
    pub fn new(
        announcements: u32,
        voters: usize, // New parameter
        last_winner: BlockHash,
        total_tally: Amount,
        final_tally: Amount, // New parameter
        blocks: HashMap<BlockHash, ConfirmationBlockInfoDto>,
    ) -> Self {
        Self {
            announcements,
            voters, // New field
            last_winner,
            total_tally,
            final_tally, // New field
            blocks,
        }
    }
}
