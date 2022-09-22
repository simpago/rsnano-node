use crate::{
    utils::{Deserialize, Serialize, Stream, StreamExt},
    BlockHash,
};

/**
 * Tag for block signature verification result
 */
#[repr(u8)]
#[derive(PartialEq, Eq, Debug, Clone, Copy, FromPrimitive)]
pub enum SignatureVerification {
    Unknown = 0,
    Invalid = 1,
    Valid = 2,
    ValidEpoch = 3, // Valid for epoch blocks
}

#[derive(Default)]
pub struct ConfirmationHeightInfo {
    pub height: u64,
    pub frontier: BlockHash,
}

impl ConfirmationHeightInfo {
    pub fn new(height: u64, frontier: BlockHash) -> Self {
        Self { height, frontier }
    }
}

impl Serialize for ConfirmationHeightInfo {
    fn serialized_size() -> usize {
        std::mem::size_of::<u64>() + BlockHash::serialized_size()
    }

    fn serialize(&self, stream: &mut dyn Stream) -> anyhow::Result<()> {
        stream.write_u64_ne(self.height)?;
        self.frontier.serialize(stream)
    }
}

impl Deserialize for ConfirmationHeightInfo {
    type Target = Self;
    fn deserialize(stream: &mut dyn Stream) -> anyhow::Result<Self> {
        let height = stream.read_u64_ne()?;
        let frontier = BlockHash::deserialize(stream)?;
        Ok(Self { height, frontier })
    }
}
