use super::MessageVariant;
use bitvec::prelude::BitArray;
use num_traits::FromPrimitive;
use rsnano_core::{
    serialized_block_size,
    utils::{BufferWriter, Serialize, Stream},
    Block, BlockType,
};
use serde_derive::Serialize;
use std::fmt::{Debug, Display};

#[derive(Clone, Eq, Serialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct Publish {
    pub block: Block,

    /// Messages deserialized from network should have their digest set
    #[serde(skip_serializing)]
    pub digest: u128,

    pub is_originator: bool,
}

impl Publish {
    const BLOCK_TYPE_MASK: u16 = 0x0f00;
    const ORIGINATOR_FLAG: u16 = 1 << 2;

    pub fn new_from_originator(block: Block) -> Self {
        Self {
            block,
            digest: 0,
            is_originator: true,
        }
    }

    pub fn new_forward(block: Block) -> Self {
        Self {
            block,
            digest: 0,
            is_originator: false,
        }
    }

    pub fn new_test_instance() -> Self {
        Self {
            block: Block::new_test_instance(),
            digest: 0,
            is_originator: true,
        }
    }

    pub fn deserialize(
        stream: &mut impl Stream,
        extensions: BitArray<u16>,
        digest: u128,
    ) -> Option<Self> {
        let payload = Publish {
            block: Block::deserialize_block_type(Self::block_type(extensions), stream).ok()?,
            digest,
            is_originator: extensions.data & Self::ORIGINATOR_FLAG > 0,
        };

        Some(payload)
    }

    pub fn serialized_size(extensions: BitArray<u16>) -> usize {
        serialized_block_size(Self::block_type(extensions))
    }

    fn block_type(extensions: BitArray<u16>) -> BlockType {
        let mut value = extensions & BitArray::new(Self::BLOCK_TYPE_MASK);
        value.shift_left(8);
        FromPrimitive::from_u16(value.data).unwrap_or(BlockType::Invalid)
    }
}

impl PartialEq for Publish {
    fn eq(&self, other: &Self) -> bool {
        self.block == other.block
    }
}

impl Serialize for Publish {
    fn serialize(&self, writer: &mut dyn BufferWriter) {
        self.block.serialize_without_block_type(writer);
    }
}

impl MessageVariant for Publish {
    fn header_extensions(&self, _payload_len: u16) -> BitArray<u16> {
        let mut flags = (self.block.block_type() as u16) << 8;
        if self.is_originator {
            flags |= Self::ORIGINATOR_FLAG;
        }
        BitArray::new(flags)
    }
}

impl Display for Publish {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "\n{}",
            self.block.to_json().map_err(|_| std::fmt::Error)?
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsnano_core::{utils::MemoryStream, BlockBuilder};

    #[test]
    fn create_from_originator() {
        let publish = Publish::new_from_originator(Block::new_test_instance());
        assert_eq!(publish.is_originator, true)
    }

    #[test]
    fn create_forward() {
        let publish = Publish::new_forward(Block::new_test_instance());
        assert_eq!(publish.is_originator, false);
    }

    #[test]
    fn originator_flag_in_header() {
        let publish = Publish::new_from_originator(Block::new_test_instance());
        let flags = publish.header_extensions(0);
        assert!(flags.data & Publish::ORIGINATOR_FLAG > 0);
    }

    #[test]
    fn originator_flag_not_in_header() {
        let publish = Publish::new_forward(Block::new_test_instance());
        let flags = publish.header_extensions(0);
        assert_eq!(flags.data & Publish::ORIGINATOR_FLAG, 0);
    }

    #[test]
    fn serialize() {
        let block = BlockBuilder::state().build();
        let mut publish1 = Publish::new_from_originator(block);
        publish1.digest = 123;

        let mut stream = MemoryStream::new();
        publish1.serialize(&mut stream);

        let extensions = publish1.header_extensions(0);
        let publish2 = Publish::deserialize(&mut stream, extensions, 123).unwrap();
        assert_eq!(publish1, publish2);
    }
}
