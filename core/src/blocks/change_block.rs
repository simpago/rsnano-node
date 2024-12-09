use super::{Block, BlockBase};
use crate::{
    utils::{BufferWriter, Deserialize, FixedSizeSerialize, Serialize, Stream},
    Account, Amount, BlockHash, BlockHashBuilder, BlockType, DependentBlocks, JsonBlock, Link,
    PrivateKey, PublicKey, Root, Signature, WorkNonce,
};
use anyhow::Result;

#[derive(Clone, Debug)]
pub struct ChangeBlock {
    work: u64,
    signature: Signature,
    hashables: ChangeHashables,
    hash: BlockHash,
}

impl ChangeBlock {
    pub fn new_test_instance() -> Self {
        let key = PrivateKey::from(42);
        ChangeBlockArgs {
            key: &key,
            previous: 123.into(),
            representative: 456.into(),
            work: 69420,
        }
        .into()
    }

    pub fn mandatory_representative(&self) -> PublicKey {
        self.hashables.representative
    }

    pub fn serialized_size() -> usize {
        ChangeHashables::serialized_size()
            + Signature::serialized_size()
            + std::mem::size_of::<u64>()
    }

    pub fn deserialize(stream: &mut dyn Stream) -> Result<Self> {
        let hashables = ChangeHashables {
            previous: BlockHash::deserialize(stream)?,
            representative: PublicKey::deserialize(stream)?,
        };

        let signature = Signature::deserialize(stream)?;
        let mut work_bytes = [0u8; 8];
        stream.read_bytes(&mut work_bytes, 8)?;
        let work = u64::from_le_bytes(work_bytes);
        let hash = hashables.hash();
        Ok(Self {
            work,
            signature,
            hashables,
            hash,
        })
    }

    pub fn dependent_blocks(&self) -> DependentBlocks {
        DependentBlocks::new(self.previous(), BlockHash::zero())
    }
}

pub fn valid_change_block_predecessor(predecessor: BlockType) -> bool {
    matches!(
        predecessor,
        BlockType::LegacySend
            | BlockType::LegacyReceive
            | BlockType::LegacyOpen
            | BlockType::LegacyChange
    )
}

impl PartialEq for ChangeBlock {
    fn eq(&self, other: &Self) -> bool {
        self.work == other.work
            && self.signature == other.signature
            && self.hashables == other.hashables
    }
}

impl Eq for ChangeBlock {}

impl BlockBase for ChangeBlock {
    fn block_type(&self) -> BlockType {
        BlockType::LegacyChange
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

    fn set_work(&mut self, work: u64) {
        self.work = work;
    }

    fn work(&self) -> u64 {
        self.work
    }

    fn set_signature(&mut self, signature: Signature) {
        self.signature = signature.clone();
    }

    fn previous(&self) -> BlockHash {
        self.hashables.previous
    }

    fn serialize_without_block_type(&self, writer: &mut dyn BufferWriter) {
        self.hashables.previous.serialize(writer);
        self.hashables.representative.serialize(writer);
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
        None
    }

    fn representative_field(&self) -> Option<PublicKey> {
        Some(self.hashables.representative)
    }

    fn valid_predecessor(&self, block_type: BlockType) -> bool {
        valid_change_block_predecessor(block_type)
    }

    fn destination_field(&self) -> Option<Account> {
        None
    }

    fn json_representation(&self) -> JsonBlock {
        JsonBlock::Change(JsonChangeBlock {
            previous: self.hashables.previous,
            representative: self.hashables.representative.into(),
            work: self.work.into(),
            signature: self.signature.clone(),
        })
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct ChangeHashables {
    previous: BlockHash,
    representative: PublicKey,
}

impl ChangeHashables {
    fn serialized_size() -> usize {
        BlockHash::serialized_size() + Account::serialized_size()
    }

    fn hash(&self) -> BlockHash {
        BlockHashBuilder::new()
            .update(self.previous.as_bytes())
            .update(self.representative.as_bytes())
            .build()
    }
}

pub struct ChangeBlockArgs<'a> {
    pub key: &'a PrivateKey,
    pub previous: BlockHash,
    pub representative: PublicKey,
    pub work: u64,
}

impl<'a> From<ChangeBlockArgs<'a>> for ChangeBlock {
    fn from(value: ChangeBlockArgs<'a>) -> Self {
        let hashables = ChangeHashables {
            previous: value.previous,
            representative: value.representative,
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

impl<'a> From<ChangeBlockArgs<'a>> for Block {
    fn from(value: ChangeBlockArgs<'a>) -> Self {
        Block::LegacyChange(value.into())
    }
}

#[derive(PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct JsonChangeBlock {
    pub previous: BlockHash,
    pub representative: Account,
    pub signature: Signature,
    pub work: WorkNonce,
}

impl From<JsonChangeBlock> for ChangeBlock {
    fn from(value: JsonChangeBlock) -> Self {
        let hashables = ChangeHashables {
            previous: value.previous,
            representative: value.representative.into(),
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
        let key1 = PrivateKey::new();
        let previous = BlockHash::from(1);
        let block: ChangeBlock = ChangeBlockArgs {
            key: &key1,
            previous,
            representative: 2.into(),
            work: 5,
        }
        .into();
        assert_eq!(block.previous(), previous);
        assert_eq!(block.root(), block.previous().into());
    }

    // original test: change_block.deserialize
    #[test]
    fn serialize() {
        let key1 = PrivateKey::new();
        let block1: ChangeBlock = ChangeBlockArgs {
            key: &key1,
            previous: 1.into(),
            representative: 2.into(),
            work: 5,
        }
        .into();
        let mut stream = MemoryStream::new();
        block1.serialize_without_block_type(&mut stream);
        assert_eq!(ChangeBlock::serialized_size(), stream.bytes_written());

        let block2 = ChangeBlock::deserialize(&mut stream).unwrap();
        assert_eq!(block1, block2);
    }

    #[test]
    fn serialize_serde() {
        let block = Block::LegacyChange(ChangeBlock::new_test_instance());
        let serialized = serde_json::to_string_pretty(&block).unwrap();
        assert_eq!(
            serialized,
            r#"{
  "type": "change",
  "previous": "000000000000000000000000000000000000000000000000000000000000007B",
  "representative": "nano_11111111111111111111111111111111111111111111111111gahteczqci",
  "signature": "6F6E98FB9C3D0B91CBAF78C8613C7A7AE990AA627B9C1381D1D97AB7118C91D169381E3897A477286A4AFB68F7CD347F3FF16F8AB4C33241D8BF793CE29E730B",
  "work": "0000000000010F2C"
}"#
        );
    }
}
