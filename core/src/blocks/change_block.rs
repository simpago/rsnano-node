use super::BlockBase;
use crate::{
    to_hex_string, u64_from_hex_str,
    utils::{BufferWriter, Deserialize, FixedSizeSerialize, PropertyTree, Serialize, Stream},
    Account, Amount, BlockHash, BlockHashBuilder, BlockType, DependentBlocks, JsonBlock,
    LazyBlockHash, Link, PrivateKey, PublicKey, Root, Signature, WorkNonce,
};
use anyhow::Result;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ChangeHashables {
    pub previous: BlockHash,
    pub representative: PublicKey,
}

impl ChangeHashables {
    fn serialized_size() -> usize {
        BlockHash::serialized_size() + Account::serialized_size()
    }
}

impl From<&ChangeHashables> for BlockHash {
    fn from(hashables: &ChangeHashables) -> Self {
        BlockHashBuilder::new()
            .update(hashables.previous.as_bytes())
            .update(hashables.representative.as_bytes())
            .build()
    }
}

#[derive(Clone, Debug)]
pub struct ChangeBlock {
    pub work: u64,
    pub signature: Signature,
    pub hashables: ChangeHashables,
    pub hash: LazyBlockHash,
}

impl ChangeBlock {
    pub fn new(
        previous: BlockHash,
        representative: PublicKey,
        prv_key: &PrivateKey,
        work: u64,
    ) -> Self {
        let hashables = ChangeHashables {
            previous,
            representative,
        };

        let hash = LazyBlockHash::new();
        let signature = prv_key.sign(hash.hash(&hashables).as_bytes());

        Self {
            work,
            signature,
            hashables,
            hash,
        }
    }

    pub fn new_test_instance() -> Self {
        let key = PrivateKey::from(42);
        Self::new(BlockHash::from(123), PublicKey::from(456), &key, 69420)
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
        Ok(Self {
            work,
            signature,
            hashables,
            hash: LazyBlockHash::new(),
        })
    }

    pub fn deserialize_json(reader: &impl PropertyTree) -> Result<Self> {
        let previous = BlockHash::decode_hex(reader.get_string("previous")?)?;
        let representative = Account::decode_account(reader.get_string("representative")?)?.into();
        let work = u64_from_hex_str(reader.get_string("work")?)?;
        let signature = Signature::decode_hex(reader.get_string("signature")?)?;
        Ok(Self {
            work,
            signature,
            hashables: ChangeHashables {
                previous,
                representative,
            },
            hash: LazyBlockHash::new(),
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
        self.hash.hash(&self.hashables)
    }

    fn link_field(&self) -> Option<Link> {
        None
    }

    fn block_signature(&self) -> &Signature {
        &self.signature
    }

    fn set_work(&mut self, work: u64) {
        self.work = work;
    }

    fn work(&self) -> u64 {
        self.work
    }

    fn set_block_signature(&mut self, signature: &Signature) {
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

    fn serialize_json(&self, writer: &mut dyn PropertyTree) -> Result<()> {
        writer.put_string("type", "change")?;
        writer.put_string("previous", &self.hashables.previous.encode_hex())?;
        writer.put_string(
            "representative",
            &Account::from(self.hashables.representative).encode_account(),
        )?;
        writer.put_string("work", &to_hex_string(self.work))?;
        writer.put_string("signature", &self.signature.encode_hex())?;
        Ok(())
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

        Self {
            work: value.work.into(),
            signature: value.signature,
            hashables,
            hash: LazyBlockHash::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        utils::{MemoryStream, TestPropertyTree},
        Block, PrivateKey,
    };

    #[test]
    fn create_block() {
        let key1 = PrivateKey::new();
        let previous = BlockHash::from(1);
        let block = ChangeBlock::new(previous.clone(), PublicKey::from(2), &key1, 5);
        assert_eq!(block.previous(), previous);
        assert_eq!(block.root(), block.previous().into());
    }

    // original test: change_block.deserialize
    #[test]
    fn serialize() {
        let key1 = PrivateKey::new();
        let block1 = ChangeBlock::new(BlockHash::from(1), PublicKey::from(2), &key1, 5);
        let mut stream = MemoryStream::new();
        block1.serialize_without_block_type(&mut stream);
        assert_eq!(ChangeBlock::serialized_size(), stream.bytes_written());

        let block2 = ChangeBlock::deserialize(&mut stream).unwrap();
        assert_eq!(block1, block2);
    }

    // original test: block.change_serialize_json
    #[test]
    fn serialize_json() {
        let block1 = ChangeBlock::new_test_instance();
        let mut ptree = TestPropertyTree::new();
        block1.serialize_json(&mut ptree).unwrap();

        let block2 = ChangeBlock::deserialize_json(&ptree).unwrap();
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
