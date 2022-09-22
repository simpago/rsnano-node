use std::mem::size_of;

use crate::{
    utils::{Deserialize, Serialize, Stream, StreamExt},
    Account, Amount, BlockHash, Epoch,
};
use anyhow::Result;
use num_traits::FromPrimitive;

/// Latest information about an account
#[derive(PartialEq, Eq, Clone, Default)]
pub struct AccountInfo {
    pub head: BlockHash,
    pub representative: Account,
    pub open_block: BlockHash,
    pub balance: Amount,
    /** Seconds since posix epoch */
    pub modified: u64,
    pub block_count: u64,
    pub epoch: Epoch,
}

impl Serialize for AccountInfo {
    fn serialized_size() -> usize {
        BlockHash::serialized_size()  // head
        + Account::serialized_size() // representative
        + BlockHash::serialized_size() // open_block
        + Amount::serialized_size() // balance
        + size_of::<u64>() // modified
        + size_of::<u64>() // block_count
        + size_of::<Epoch>()
    }

    fn serialize(&self, stream: &mut dyn Stream) -> Result<()> {
        self.head.serialize(stream)?;
        self.representative.serialize(stream)?;
        self.open_block.serialize(stream)?;
        self.balance.serialize(stream)?;
        stream.write_u64_ne(self.modified)?;
        stream.write_u64_ne(self.block_count)?;
        stream.write_u8(self.epoch as u8)
    }
}

impl Deserialize for AccountInfo {
    type Target = Self;
    fn deserialize(stream: &mut dyn Stream) -> Result<AccountInfo> {
        Ok(Self {
            head: BlockHash::deserialize(stream)?,
            representative: Account::deserialize(stream)?,
            open_block: BlockHash::deserialize(stream)?,
            balance: Amount::deserialize(stream)?,
            modified: stream.read_u64_ne()?,
            block_count: stream.read_u64_ne()?,
            epoch: Epoch::from_u8(stream.read_u8()?).ok_or_else(|| anyhow!("invalid epoch"))?,
        })
    }
}
