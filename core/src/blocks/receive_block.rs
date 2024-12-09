use super::{Block, BlockBase, BlockType};
use crate::{
    utils::{BufferWriter, Deserialize, FixedSizeSerialize, Serialize, Stream},
    Account, Amount, BlockHash, BlockHashBuilder, DependentBlocks, JsonBlock, Link, PrivateKey,
    PublicKey, Root, Signature, WorkNonce,
};
use anyhow::Result;

#[derive(Clone, Debug)]
pub struct ReceiveBlock {
    work: u64,
    signature: Signature,
    hashables: ReceiveHashables,
    hash: BlockHash,
}

impl ReceiveBlock {
    pub fn new_test_instance() -> Self {
        let key = PrivateKey::from(42);
        ReceiveBlockArgs {
            key: &key,
            previous: 123.into(),
            source: 456.into(),
            work: 69420,
        }
        .into()
    }

    // Receive blocks always have a source
    pub fn source(&self) -> BlockHash {
        self.hashables.source
    }

    pub fn serialized_size() -> usize {
        ReceiveHashables::serialized_size()
            + Signature::serialized_size()
            + std::mem::size_of::<u64>()
    }

    pub fn deserialize(stream: &mut dyn Stream) -> Result<Self> {
        let previous = BlockHash::deserialize(stream)?;
        let source = BlockHash::deserialize(stream)?;
        let signature = Signature::deserialize(stream)?;
        let mut work_bytes = [0u8; 8];
        stream.read_bytes(&mut work_bytes, 8)?;
        let work = u64::from_le_bytes(work_bytes);
        let hashables = ReceiveHashables { previous, source };
        let hash = hashables.hash();
        Ok(Self {
            work,
            signature,
            hashables,
            hash,
        })
    }

    pub fn dependent_blocks(&self) -> DependentBlocks {
        DependentBlocks::new(self.previous(), self.source())
    }
}

pub fn valid_receive_block_predecessor(predecessor: BlockType) -> bool {
    matches!(
        predecessor,
        BlockType::LegacySend
            | BlockType::LegacyReceive
            | BlockType::LegacyOpen
            | BlockType::LegacyChange
    )
}

impl PartialEq for ReceiveBlock {
    fn eq(&self, other: &Self) -> bool {
        self.work == other.work
            && self.signature == other.signature
            && self.hashables == other.hashables
    }
}

impl Eq for ReceiveBlock {}

impl BlockBase for ReceiveBlock {
    fn block_type(&self) -> BlockType {
        BlockType::LegacyReceive
    }

    fn account_field(&self) -> Option<Account> {
        None
    }

    fn hash(&self) -> BlockHash {
        self.hash
    }

    fn link_field(&self) -> Option<Link> {
        None
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn set_signature(&mut self, signature: Signature) {
        self.signature = signature;
    }

    fn set_work(&mut self, work: u64) {
        self.work = work;
    }

    fn work(&self) -> u64 {
        self.work
    }

    fn previous(&self) -> BlockHash {
        self.hashables.previous
    }

    fn serialize_without_block_type(&self, writer: &mut dyn BufferWriter) {
        self.hashables.previous.serialize(writer);
        self.hashables.source.serialize(writer);
        self.signature.serialize(writer);
        writer.write_bytes_safe(&self.work.to_le_bytes());
    }

    fn root(&self) -> Root {
        self.previous().into()
    }

    fn balance_field(&self) -> Option<Amount> {
        None
    }

    fn source_field(&self) -> Option<BlockHash> {
        Some(self.hashables.source)
    }

    fn representative_field(&self) -> Option<PublicKey> {
        None
    }

    fn valid_predecessor(&self, block_type: BlockType) -> bool {
        valid_receive_block_predecessor(block_type)
    }

    fn destination_field(&self) -> Option<Account> {
        None
    }

    fn json_representation(&self) -> JsonBlock {
        JsonBlock::Receive(JsonReceiveBlock {
            previous: self.hashables.previous,
            source: self.hashables.source,
            work: self.work.into(),
            signature: self.signature.clone(),
        })
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct ReceiveHashables {
    previous: BlockHash,
    source: BlockHash,
}

impl ReceiveHashables {
    fn serialized_size() -> usize {
        BlockHash::serialized_size() + BlockHash::serialized_size()
    }

    fn hash(&self) -> BlockHash {
        BlockHashBuilder::new()
            .update(self.previous.as_bytes())
            .update(self.source.as_bytes())
            .build()
    }
}

pub struct ReceiveBlockArgs<'a> {
    pub key: &'a PrivateKey,
    pub previous: BlockHash,
    pub source: BlockHash,
    pub work: u64,
}

impl<'a> From<ReceiveBlockArgs<'a>> for ReceiveBlock {
    fn from(value: ReceiveBlockArgs<'a>) -> Self {
        let hashables = ReceiveHashables {
            previous: value.previous,
            source: value.source,
        };
        let hash = hashables.hash();
        let signature = value.key.sign(hash.as_bytes());

        Self {
            work: value.work,
            signature,
            hashables,
            hash,
        }
    }
}

impl<'a> From<ReceiveBlockArgs<'a>> for Block {
    fn from(value: ReceiveBlockArgs<'a>) -> Self {
        Block::LegacyReceive(value.into())
    }
}

#[derive(PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct JsonReceiveBlock {
    pub previous: BlockHash,
    pub source: BlockHash,
    pub signature: Signature,
    pub work: WorkNonce,
}

impl From<JsonReceiveBlock> for ReceiveBlock {
    fn from(value: JsonReceiveBlock) -> Self {
        let hashables = ReceiveHashables {
            previous: value.previous,
            source: value.source,
        };
        let hash = hashables.hash();

        Self {
            work: value.work.into(),
            signature: value.signature,
            hashables,
            hash,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{utils::MemoryStream, Block, PrivateKey};

    #[test]
    fn create_block() {
        let key = PrivateKey::new();
        let previous = BlockHash::from(1);
        let block: ReceiveBlock = ReceiveBlockArgs {
            key: &key,
            previous,
            source: 2.into(),
            work: 4,
        }
        .into();
        assert_eq!(block.previous(), previous);
        assert_eq!(block.root(), previous.into());
    }

    // original test: block.receive_serialize
    // original test: receive_block.deserialize
    #[test]
    fn serialize() {
        let key1 = PrivateKey::new();
        let block1: ReceiveBlock = ReceiveBlockArgs {
            key: &key1,
            previous: 0.into(),
            source: 1.into(),
            work: 4,
        }
        .into();
        let mut stream = MemoryStream::new();
        block1.serialize_without_block_type(&mut stream);
        assert_eq!(ReceiveBlock::serialized_size(), stream.bytes_written());

        let block2 = ReceiveBlock::deserialize(&mut stream).unwrap();
        assert_eq!(block1, block2);
    }

    #[test]
    fn serialize_serde() {
        let block = Block::LegacyReceive(ReceiveBlock::new_test_instance());
        let serialized = serde_json::to_string_pretty(&block).unwrap();
        assert_eq!(
            serialized,
            r#"{
  "type": "receive",
  "previous": "000000000000000000000000000000000000000000000000000000000000007B",
  "source": "00000000000000000000000000000000000000000000000000000000000001C8",
  "signature": "6F6E98FB9C3D0B91CBAF78C8613C7A7AE990AA627B9C1381D1D97AB7118C91D169381E3897A477286A4AFB68F7CD347F3FF16F8AB4C33241D8BF793CE29E730B",
  "work": "0000000000010F2C"
}"#
        );
    }
}
