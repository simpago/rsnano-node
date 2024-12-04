use super::MessageVariant;
use bitvec::prelude::BitArray;
use rsnano_core::{
    utils::{BufferWriter, Deserialize, FixedSizeSerialize, Serialize, Stream},
    BlockHash, HashOrAccount,
};
use serde_derive::Serialize;
use std::{fmt::Display, mem::size_of};

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct BulkPull {
    pub start: HashOrAccount,
    pub end: BlockHash,
    pub count: u32,
    pub ascending: bool,
}

impl BulkPull {
    pub const COUNT_PRESENT_FLAG: usize = 0;
    pub const ASCENDING_FLAG: usize = 1;
    pub const EXTENDED_PARAMETERS_SIZE: usize = 8;

    pub fn new_test_instance() -> BulkPull {
        Self {
            start: 1.into(),
            end: 2.into(),
            count: 3,
            ascending: true,
        }
    }

    pub fn serialized_size(extensions: BitArray<u16>) -> usize {
        HashOrAccount::serialized_size()
            + BlockHash::serialized_size()
            + (if extensions[BulkPull::COUNT_PRESENT_FLAG] {
                BulkPull::EXTENDED_PARAMETERS_SIZE
            } else {
                0
            })
    }

    pub fn deserialize(stream: &mut impl Stream, extensions: BitArray<u16>) -> Option<Self> {
        let start = HashOrAccount::deserialize(stream).ok()?;
        let end = BlockHash::deserialize(stream).ok()?;

        let count = if extensions[BulkPull::COUNT_PRESENT_FLAG] {
            let mut extended_parameters_buffers = [0u8; BulkPull::EXTENDED_PARAMETERS_SIZE];
            const_assert!(size_of::<u32>() < (BulkPull::EXTENDED_PARAMETERS_SIZE - 1)); // "count must fit within buffer")

            stream
                .read_bytes(
                    &mut extended_parameters_buffers,
                    BulkPull::EXTENDED_PARAMETERS_SIZE,
                )
                .ok()?;
            if extended_parameters_buffers[0] != 0 {
                return None;
            } else {
                u32::from_le_bytes(extended_parameters_buffers[1..5].try_into().unwrap())
            }
        } else {
            0
        };

        let ascending = extensions[BulkPull::ASCENDING_FLAG];

        Some(BulkPull {
            start,
            end,
            count,
            ascending,
        })
    }
}

impl Serialize for BulkPull {
    fn serialize(&self, writer: &mut dyn BufferWriter) {
        self.start.serialize(writer);
        self.end.serialize(writer);

        if self.count > 0 {
            let mut count_buffer = [0u8; BulkPull::EXTENDED_PARAMETERS_SIZE];
            const_assert!(size_of::<u32>() < (BulkPull::EXTENDED_PARAMETERS_SIZE - 1)); // count must fit within buffer

            count_buffer[1..5].copy_from_slice(&self.count.to_le_bytes());
            writer.write_bytes_safe(&count_buffer);
        }
    }
}

impl MessageVariant for BulkPull {
    fn header_extensions(&self, _payload_len: u16) -> BitArray<u16> {
        let mut extensions = BitArray::default();
        extensions.set(BulkPull::COUNT_PRESENT_FLAG, self.count > 0);
        extensions.set(BulkPull::ASCENDING_FLAG, self.ascending);
        extensions
    }
}

impl Display for BulkPull {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "\nstart={} end={} cnt={}",
            self.start, self.end, self.count
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{assert_deserializable, Message};

    #[test]
    fn bulk_pull_serialization() {
        let message = Message::BulkPull(BulkPull::new_test_instance());
        assert_deserializable(&message);
    }
}
