use crate::{
    utils::{BufferWriter, Deserialize, FixedSizeSerialize, Serialize, Stream},
    Account, Amount, BlockDetails, BlockHash, BlockType, Epoch,
};
use num::FromPrimitive;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockSideband {
    pub height: u64,
    pub timestamp: u64,
    /// Successor to the current block
    pub successor: BlockHash,
    pub account: Account,
    pub balance: Amount,
    pub details: BlockDetails,
    pub source_epoch: Epoch,
}

impl BlockSideband {
    pub fn serialized_size(block_type: BlockType) -> usize {
        let mut size = BlockHash::serialized_size(); // successor

        if block_type != BlockType::State && block_type != BlockType::LegacyOpen {
            size += Account::serialized_size(); // account
        }

        if block_type != BlockType::LegacyOpen {
            size += std::mem::size_of::<u64>(); // height
        }

        if block_type == BlockType::LegacyReceive
            || block_type == BlockType::LegacyChange
            || block_type == BlockType::LegacyOpen
        {
            size += Amount::serialized_size(); // balance
        }

        size += std::mem::size_of::<u64>(); // timestamp

        if block_type == BlockType::State {
            // block_details must not be larger than the epoch enum
            const_assert!(std::mem::size_of::<Epoch>() == BlockDetails::serialized_size());
            size += BlockDetails::serialized_size() + std::mem::size_of::<Epoch>();
        }

        size
    }

    pub fn serialize(&self, stream: &mut dyn BufferWriter, block_type: BlockType) {
        self.successor.serialize(stream);

        if block_type != BlockType::State && block_type != BlockType::LegacyOpen {
            self.account.serialize(stream);
        }

        if block_type != BlockType::LegacyOpen {
            stream.write_bytes_safe(&self.height.to_be_bytes());
        }

        if block_type == BlockType::LegacyReceive
            || block_type == BlockType::LegacyChange
            || block_type == BlockType::LegacyOpen
        {
            self.balance.serialize(stream);
        }

        stream.write_bytes_safe(&self.timestamp.to_be_bytes());

        if block_type == BlockType::State {
            self.details.serialize(stream);
            stream.write_u8_safe(self.source_epoch as u8);
        }
    }

    pub fn from_stream(stream: &mut dyn Stream, block_type: BlockType) -> anyhow::Result<Self> {
        let mut result = Self {
            height: 0,
            timestamp: 0,
            successor: BlockHash::zero(),
            account: Account::zero(),
            balance: Amount::zero(),
            details: BlockDetails::new(Epoch::Epoch0, false, false, false),
            source_epoch: Epoch::Epoch0,
        };
        result.deserialize(stream, block_type)?;
        Ok(result)
    }

    pub fn deserialize(
        &mut self,
        stream: &mut dyn Stream,
        block_type: BlockType,
    ) -> anyhow::Result<()> {
        self.successor = BlockHash::deserialize(stream)?;

        if block_type != BlockType::State && block_type != BlockType::LegacyOpen {
            self.account = Account::deserialize(stream)?;
        }

        let mut buffer = [0u8; 8];
        if block_type != BlockType::LegacyOpen {
            stream.read_bytes(&mut buffer, 8)?;
            self.height = u64::from_be_bytes(buffer);
        } else {
            self.height = 1;
        }

        if block_type == BlockType::LegacyReceive
            || block_type == BlockType::LegacyChange
            || block_type == BlockType::LegacyOpen
        {
            self.balance = Amount::deserialize(stream)?;
        }

        stream.read_bytes(&mut buffer, 8)?;
        self.timestamp = u64::from_be_bytes(buffer);

        if block_type == BlockType::State {
            self.details = BlockDetails::deserialize(stream)?;
            self.source_epoch = FromPrimitive::from_u8(stream.read_u8()?)
                .ok_or_else(|| anyhow!("invalid epoch value"))?;
        }

        Ok(())
    }

    pub fn new_test_instance() -> Self {
        Self {
            height: 42,
            timestamp: 1000,
            successor: BlockHash::from(3),
            account: Account::from(1),
            balance: Amount::raw(42),
            details: BlockDetails {
                epoch: Epoch::Epoch2,
                is_send: true,
                is_receive: false,
                is_epoch: false,
            },
            source_epoch: Epoch::Epoch2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::MemoryStream;

    #[test]
    fn serialize() {
        let details = BlockDetails::new(Epoch::Epoch0, false, false, false);
        let sideband = BlockSideband {
            height: 4,
            timestamp: 5,
            successor: 2.into(),
            account: 1.into(),
            balance: 3.into(),
            details,
            source_epoch: Epoch::Epoch0,
        };
        let mut stream = MemoryStream::new();
        sideband.serialize(&mut stream, BlockType::LegacyReceive);
        let deserialized =
            BlockSideband::from_stream(&mut stream, BlockType::LegacyReceive).unwrap();
        assert_eq!(deserialized, sideband);
    }
}
