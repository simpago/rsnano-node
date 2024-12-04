use super::BlockHash;
use crate::{
    utils::{
        BufferWriter, Deserialize, FixedSizeSerialize, MemoryStream, Serialize, Stream, StreamExt,
    },
    Block,
};
use std::time::{SystemTime, UNIX_EPOCH};

/// Information on an unchecked block
#[derive(Clone, Debug)]
pub struct UncheckedInfo {
    pub block: Block,

    /// Seconds since posix epoch
    pub modified: u64,
}

impl UncheckedInfo {
    pub fn new(block: Block) -> Self {
        Self {
            block,
            modified: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut stream = MemoryStream::new();
        self.serialize(&mut stream);
        stream.to_vec()
    }
}

impl Serialize for UncheckedInfo {
    fn serialize(&self, stream: &mut dyn BufferWriter) {
        self.block.serialize(stream);
        stream.write_u64_ne_safe(self.modified);
    }
}

impl Deserialize for UncheckedInfo {
    type Target = Self;

    fn deserialize(stream: &mut dyn Stream) -> anyhow::Result<Self::Target> {
        let block = Block::deserialize(stream)?;
        let modified = stream.read_u64_ne()?;
        Ok(Self { block, modified })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UncheckedKey {
    pub previous: BlockHash,
    pub hash: BlockHash,
}

impl UncheckedKey {
    pub fn new(previous: BlockHash, hash: BlockHash) -> Self {
        Self { previous, hash }
    }

    pub fn to_bytes(&self) -> [u8; 64] {
        let mut result = [0; 64];
        result[..32].copy_from_slice(self.previous.as_bytes());
        result[32..].copy_from_slice(self.hash.as_bytes());
        result
    }
}

impl Deserialize for UncheckedKey {
    type Target = Self;

    fn deserialize(stream: &mut dyn Stream) -> anyhow::Result<Self::Target> {
        let previous = BlockHash::deserialize(stream)?;
        let hash = BlockHash::deserialize(stream)?;
        Ok(Self { previous, hash })
    }
}

impl Serialize for UncheckedKey {
    fn serialize(&self, writer: &mut dyn BufferWriter) {
        self.previous.serialize(writer);
        self.hash.serialize(writer);
    }
}

impl FixedSizeSerialize for UncheckedKey {
    fn serialized_size() -> usize {
        BlockHash::serialized_size() * 2
    }
}
