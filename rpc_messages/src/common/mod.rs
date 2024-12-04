mod account;
mod accounts;
mod accounts_with_amounts;
mod address_with_port;
mod amount;
mod block;
mod blocks;
mod count;
mod destroyed;
mod error;
mod exists;
mod frontiers;
mod hash;
mod hashes;
mod key_pair;
mod locked;
mod moved;
mod primitives;
mod public_key;
mod removed;
mod started;
mod success;
mod valid;
mod wallet;

pub use account::*;
pub use accounts::*;
pub use accounts_with_amounts::*;
pub use address_with_port::*;
pub use amount::*;
pub use block::*;
pub use blocks::*;
pub use count::*;
pub use destroyed::*;
pub use error::*;
pub use exists::*;
pub use frontiers::*;
pub use hash::*;
pub use hashes::*;
pub use key_pair::*;
pub use locked::*;
pub use moved::*;
pub use primitives::*;
pub use public_key::*;
pub use removed::*;
pub use started::*;
pub use success::*;
pub use valid::*;
pub use wallet::*;

use rsnano_core::{BlockSubType, BlockType, WorkVersion};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkVersionDto {
    Work1,
}

impl From<WorkVersion> for WorkVersionDto {
    fn from(value: WorkVersion) -> Self {
        match value {
            WorkVersion::Unspecified => unimplemented!(),
            WorkVersion::Work1 => WorkVersionDto::Work1,
        }
    }
}

impl From<WorkVersionDto> for WorkVersion {
    fn from(value: WorkVersionDto) -> Self {
        match value {
            WorkVersionDto::Work1 => WorkVersion::Work1,
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockTypeDto {
    Send,
    Receive,
    Open,
    Change,
    State,
    Unknown,
}

impl From<BlockType> for BlockTypeDto {
    fn from(value: BlockType) -> Self {
        match value {
            BlockType::LegacySend => BlockTypeDto::Send,
            BlockType::LegacyReceive => BlockTypeDto::Receive,
            BlockType::LegacyOpen => BlockTypeDto::Open,
            BlockType::LegacyChange => BlockTypeDto::Change,
            BlockType::State => BlockTypeDto::State,
            BlockType::Invalid | BlockType::NotABlock => BlockTypeDto::Unknown,
        }
    }
}

impl From<BlockTypeDto> for BlockType {
    fn from(value: BlockTypeDto) -> Self {
        match value {
            BlockTypeDto::Send => BlockType::LegacySend,
            BlockTypeDto::Receive => BlockType::LegacyReceive,
            BlockTypeDto::Open => BlockType::LegacyOpen,
            BlockTypeDto::Change => BlockType::LegacyChange,
            BlockTypeDto::State => BlockType::State,
            BlockTypeDto::Unknown => BlockType::Invalid,
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockSubTypeDto {
    Send,
    Receive,
    Open,
    Change,
    Epoch,
    Unknown,
}

impl From<BlockSubType> for BlockSubTypeDto {
    fn from(value: BlockSubType) -> Self {
        match value {
            BlockSubType::Send => Self::Send,
            BlockSubType::Receive => Self::Receive,
            BlockSubType::Open => Self::Open,
            BlockSubType::Change => Self::Change,
            BlockSubType::Epoch => Self::Epoch,
        }
    }
}
