use super::MessageVariant;
use anyhow::Result;
use bitvec::prelude::BitArray;
use num_traits::FromPrimitive;
use rsnano_core::{
    serialized_block_size,
    utils::{BufferWriter, Deserialize, FixedSizeSerialize, Serialize, Stream},
    BlockHash, BlockType, Root,
};
use serde::ser::{SerializeSeq, SerializeStruct};
use std::fmt::{Debug, Display, Write};

/*
 * Binary Format:
 * [message_header] Common message header
 * [N x (32 bytes (block hash) + 32 bytes (root))] Pairs of (block_hash, root)
 * - The count is determined by the header's count bits.
 *
 * Header extensions:
 * - [0xf000] Count (for V1 protocol)
 * - [0x0f00] Block type
 *   - Not used anymore (V25.1+), but still present and set to `not_a_block = 0x1` for backwards compatibility
 * - [0xf000 (high), 0x00f0 (low)] Count V2 (for V2 protocol)
 * - [0x0001] Confirm V2 flag
 * - [0x0002] Reserved for V3+ versioning
 */
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ConfirmReq {
    pub roots_hashes: Vec<(BlockHash, Root)>,
}

impl ConfirmReq {
    pub const HASHES_MAX: usize = 255;

    // Header extension bits:
    // ----------------------
    const COUNT_HIGH_MASK: u16 = 0b1111_0000_0000_0000;
    const COUNT_HIGH_SHIFT: u16 = 12;
    const BLOCK_TYPE_MASK: u16 = 0b0000_1111_0000_0000;
    const BLOCK_TYPE_SHIFT: u16 = 8;
    const COUNT_LOW_MASK: u16 = 0b0000_0000_1111_0000;
    const COUNT_LOW_SHIFT: u16 = 4;
    const V2_FLAG: u16 = 0b0000_0000_0000_0001;
    // ----------------------

    pub fn new(roots_hashes: Vec<(BlockHash, Root)>) -> Self {
        if roots_hashes.len() > u8::MAX as usize {
            panic!("roots_hashes too big");
        }
        Self { roots_hashes }
    }

    pub fn new_test_instance() -> Self {
        Self::new(vec![(BlockHash::from(123), Root::from(456))])
    }

    pub fn roots_hashes(&self) -> &Vec<(BlockHash, Root)> {
        &self.roots_hashes
    }

    fn block_type(extensions: BitArray<u16>) -> BlockType {
        let value = (extensions.data & Self::BLOCK_TYPE_MASK) >> Self::BLOCK_TYPE_SHIFT;
        FromPrimitive::from_u16(value).unwrap_or(BlockType::Invalid)
    }

    pub fn count(extensions: BitArray<u16>) -> u8 {
        if Self::has_v2_flag(extensions) {
            Self::v2_count(extensions)
        } else {
            Self::v1_count(extensions)
        }
    }

    pub fn deserialize(stream: &mut impl Stream, extensions: BitArray<u16>) -> Option<Self> {
        Some(Self::new(Self::deserialize_roots(stream, extensions).ok()?))
    }

    fn deserialize_roots(
        stream: &mut impl Stream,
        extensions: BitArray<u16>,
    ) -> Result<Vec<(BlockHash, Root)>> {
        let count = Self::count(extensions) as usize;
        let mut roots_hashes = Vec::with_capacity(count);
        for _ in 0..count {
            let block_hash = BlockHash::deserialize(stream)?;
            let root = Root::deserialize(stream)?;
            if !block_hash.is_zero() || !root.is_zero() {
                roots_hashes.push((block_hash, root));
            }
        }

        if roots_hashes.is_empty() || roots_hashes.len() != count {
            bail!("roots hashes empty or incorrect count");
        }

        Ok(roots_hashes)
    }

    pub fn roots_string(&self) -> String {
        let mut result = String::new();
        for (hash, root) in &self.roots_hashes {
            write!(&mut result, "{}:{}, ", hash, root).unwrap();
        }
        result
    }

    pub fn serialized_size(extensions: BitArray<u16>) -> usize {
        let count = Self::count(extensions);
        let mut result = 0;
        let block_type = Self::block_type(extensions);
        if block_type != BlockType::Invalid && block_type != BlockType::NotABlock {
            result = serialized_block_size(block_type);
        } else if block_type == BlockType::NotABlock {
            result = count as usize * (BlockHash::serialized_size() + Root::serialized_size());
        }
        result
    }

    pub fn count_bits(count: u8) -> BitArray<u16> {
        // We need those shenanigans because we need to keep compatibility with previous protocol versions (<= V25.1)
        //
        // V1:
        // 0b{CCCC}_0000_0000_0000
        //  C: count bits
        //
        // V2:
        // 0b{HHHH}_0000_{LLLL}_000{F}
        //  H: count high bits
        //  L: count low bits
        //  F: v2 flag
        if count < 16 {
            // v1. Allows 4 bits
            BitArray::new((count as u16) << Self::COUNT_HIGH_SHIFT)
        } else {
            // v2. Allows 8 bits
            let mut bits = 0u16;
            let (left, right) = Self::split_count(count);
            bits |= Self::V2_FLAG;
            bits |= (left as u16) << Self::COUNT_HIGH_SHIFT;
            bits |= (right as u16) << Self::COUNT_LOW_SHIFT;
            BitArray::new(bits)
        }
    }

    /// Splits the count into two 4-bit parts
    fn split_count(count: u8) -> (u8, u8) {
        let left = (count >> 4) & 0xf;
        let right = count & 0xf;
        (left, right)
    }

    fn has_v2_flag(bits: BitArray<u16>) -> bool {
        bits.data & Self::V2_FLAG == Self::V2_FLAG
    }

    fn v1_count(bits: BitArray<u16>) -> u8 {
        ((bits.data & Self::COUNT_HIGH_MASK) >> Self::COUNT_HIGH_SHIFT) as u8
    }

    fn v2_count(bits: BitArray<u16>) -> u8 {
        let left = (bits.data & Self::COUNT_HIGH_MASK) >> Self::COUNT_HIGH_SHIFT;
        let right = (bits.data & Self::COUNT_LOW_MASK) >> Self::COUNT_LOW_SHIFT;
        ((left << 4) | right) as u8
    }
}

impl Serialize for ConfirmReq {
    fn serialize(&self, writer: &mut dyn BufferWriter) {
        for (hash, root) in &self.roots_hashes {
            writer.write_bytes_safe(hash.as_bytes());
            writer.write_bytes_safe(root.as_bytes());
        }
    }
}

struct SerializableRootsHashes<'a>(&'a Vec<(BlockHash, Root)>);

impl<'a> serde::Serialize for SerializableRootsHashes<'a> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for item in self.0.iter() {
            seq.serialize_element(&SerializableRootHash(item))?;
        }
        seq.end()
    }
}

struct SerializableRootHash<'a>(&'a (BlockHash, Root));

impl<'a> serde::Serialize for SerializableRootHash<'a> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_struct("RootHash", 2)?;
        seq.serialize_field("hash", &self.0 .0)?;
        seq.serialize_field("root", &self.0 .1.encode_hex())?;
        seq.end()
    }
}

impl serde::Serialize for ConfirmReq {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("ConfirmReq", 6)?;
        state.serialize_field("confirm_type", "roots_hashes")?;
        state.serialize_field("roots_hashes", &SerializableRootsHashes(&self.roots_hashes))?;
        state.end()
    }
}

impl MessageVariant for ConfirmReq {
    fn header_extensions(&self, _payload_len: u16) -> BitArray<u16> {
        let mut extensions = BitArray::default();
        extensions |= Self::count_bits(self.roots_hashes.len() as u8);
        // Set NotABlock (1) block type for hashes + roots request
        // This is needed to keep compatibility with previous protocol versions (<= V25.1)
        extensions |= BitArray::new((BlockType::NotABlock as u16) << Self::BLOCK_TYPE_SHIFT);
        extensions
    }
}

impl Display for ConfirmReq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (hash, root) in &self.roots_hashes {
            write!(f, "\n{}:{}", hash, root)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{assert_deserializable, Message};

    #[test]
    fn serialize() {
        let confirm_req = Message::ConfirmReq(ConfirmReq::new_test_instance());
        assert_deserializable(&confirm_req);
    }

    #[test]
    fn serialize_many_roots_hashes() {
        let roots_hashes = (0..7)
            .into_iter()
            .map(|i| (BlockHash::from(i), Root::from(i + 1)))
            .collect();
        let confirm_req = Message::ConfirmReq(ConfirmReq::new(roots_hashes));
        assert_deserializable(&confirm_req);
    }

    #[test]
    fn get_block_type_from_header() {
        let extensions = Default::default();
        assert_eq!(ConfirmReq::block_type(extensions), BlockType::Invalid);

        let confirm_req = ConfirmReq::new_test_instance();
        let extensions = confirm_req.header_extensions(0);
        assert_eq!(ConfirmReq::block_type(extensions), BlockType::NotABlock);
    }

    #[test]
    fn v1_extensions() {
        let confirm_req = ConfirmReq::new(vec![(BlockHash::from(1), Root::from(2)); 15]);
        let extensions = confirm_req.header_extensions(0);
        // count=15 plus NotABlock flag
        let expected = 0b_1111_0001_0000_0000;
        assert_eq!(extensions.data, expected);
    }

    #[test]
    fn use_v2_with_16_roots() {
        let confirm_req = ConfirmReq::new(vec![(BlockHash::from(1), Root::from(2)); 16]);
        let extensions = confirm_req.header_extensions(0);
        // count=16 plus NotABlock flag plus v2 flag
        let expected = 0b_0001_0001_0000_0001;
        assert_eq!(extensions.data, expected);
    }

    #[test]
    fn split_count() {
        assert_eq!(ConfirmReq::split_count(0b0), (0b0, 0b0));
        assert_eq!(ConfirmReq::split_count(0b1), (0b0, 0b1));
        assert_eq!(ConfirmReq::split_count(0b11), (0b0, 0b11));
        assert_eq!(ConfirmReq::split_count(0b111), (0b0, 0b111));
        assert_eq!(ConfirmReq::split_count(0b1111), (0b0, 0b1111));
        assert_eq!(ConfirmReq::split_count(0b11111), (0b1, 0b1111));
        assert_eq!(ConfirmReq::split_count(0b111111), (0b11, 0b1111));
        assert_eq!(ConfirmReq::split_count(0b10101010), (0b1010, 0b1010));
    }

    #[test]
    fn extract_v1_count() {
        assert_eq!(ConfirmReq::count(BitArray::new(0)), 0);
        assert_eq!(ConfirmReq::count(BitArray::new(0b0001_0000_0000_0000)), 1);
        assert_eq!(ConfirmReq::count(BitArray::new(0b1010_0000_0000_0000)), 10);
        assert_eq!(ConfirmReq::count(BitArray::new(0b1111_0000_0000_0000)), 15);
        assert_eq!(ConfirmReq::count(BitArray::new(0b1111_0000_1111_0000)), 15);
    }

    #[test]
    fn extract_v2_count() {
        assert_eq!(ConfirmReq::count(BitArray::new(0b0000_0000_0000_0001)), 0);
        assert_eq!(ConfirmReq::count(BitArray::new(0b0000_0000_1010_0001)), 10);
        assert_eq!(ConfirmReq::count(BitArray::new(0b0000_0000_1111_0001)), 15);
        assert_eq!(ConfirmReq::count(BitArray::new(0b0001_0000_0000_0001)), 16);
        assert_eq!(ConfirmReq::count(BitArray::new(0b1111_0000_0001_0001)), 241);
        assert_eq!(ConfirmReq::count(BitArray::new(0b1111_0000_1111_0001)), 255);
    }

    #[test]
    fn v2_extensions() {
        let confirm_req = ConfirmReq::new(vec![(BlockHash::from(1), Root::from(2)); 0b10001010]);
        let extensions = confirm_req.header_extensions(0);
        let expected = 0b1000_0001_1010_0001;
        assert_eq!(extensions.data, expected);
    }

    #[test]
    fn serialize_v2() {
        let confirm_req =
            Message::ConfirmReq(ConfirmReq::new(vec![
                (BlockHash::from(1), Root::from(2));
                255
            ]));
        assert_deserializable(&confirm_req);
    }

    #[test]
    #[should_panic]
    fn panics_when_roots_hashes_are_too_big() {
        ConfirmReq::new(vec![(BlockHash::from(1), Root::from(2)); 256]);
    }
}
