use super::{Block, BlockSideband, BlockType, BlockVisitor};
use crate::{
    sign_message, to_hex_string, u64_from_hex_str,
    utils::{BufferWriter, Deserialize, FixedSizeSerialize, PropertyTree, Serialize, Stream},
    Account, Amount, BlockHash, BlockHashBuilder, JsonBlock, KeyPair, LazyBlockHash, Link,
    PublicKey, RawKey, Root, Signature, WorkNonce,
};
use anyhow::Result;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ReceiveHashables {
    pub previous: BlockHash,
    pub source: BlockHash,
}

impl ReceiveHashables {
    fn serialized_size() -> usize {
        BlockHash::serialized_size() + BlockHash::serialized_size()
    }
}

impl From<&ReceiveHashables> for BlockHash {
    fn from(hashables: &ReceiveHashables) -> Self {
        BlockHashBuilder::new()
            .update(hashables.previous.as_bytes())
            .update(hashables.source.as_bytes())
            .build()
    }
}

#[derive(Clone, Debug)]
pub struct ReceiveBlock {
    pub work: u64,
    pub signature: Signature,
    pub hashables: ReceiveHashables,
    pub hash: LazyBlockHash,
    pub sideband: Option<BlockSideband>,
}

impl ReceiveBlock {
    pub fn new(
        previous: BlockHash,
        source: BlockHash,
        priv_key: &RawKey,
        pub_key: &PublicKey,
        work: u64,
    ) -> Self {
        let hashables = ReceiveHashables { previous, source };
        let hash = LazyBlockHash::new();
        let signature = sign_message(priv_key, pub_key, hash.hash(&hashables).as_bytes());

        Self {
            work,
            signature,
            hashables,
            hash,
            sideband: None,
        }
    }

    pub fn new_test_instance() -> Self {
        let key = KeyPair::from(42);
        ReceiveBlock::new(
            BlockHash::from(123),
            BlockHash::from(456),
            &key.private_key(),
            &key.public_key(),
            69420,
        )
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

    pub fn deserialize_json(reader: &impl PropertyTree) -> Result<Self> {
        let previous = BlockHash::decode_hex(reader.get_string("previous")?)?;
        let source = BlockHash::decode_hex(reader.get_string("source")?)?;
        let signature = Signature::decode_hex(reader.get_string("signature")?)?;
        let work = u64_from_hex_str(reader.get_string("work")?)?;
        Ok(Self {
            work,
            signature,
            hashables: ReceiveHashables { previous, source },
            hash: LazyBlockHash::new(),
            sideband: None,
        })
    }

    pub fn deserialize(stream: &mut dyn Stream) -> Result<Self> {
        let previous = BlockHash::deserialize(stream)?;
        let source = BlockHash::deserialize(stream)?;
        let signature = Signature::deserialize(stream)?;
        let mut work_bytes = [0u8; 8];
        stream.read_bytes(&mut work_bytes, 8)?;
        let work = u64::from_be_bytes(work_bytes);
        Ok(Self {
            work,
            signature,
            hashables: ReceiveHashables { previous, source },
            hash: LazyBlockHash::new(),
            sideband: None,
        })
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

impl Block for ReceiveBlock {
    fn sideband(&'_ self) -> Option<&'_ BlockSideband> {
        self.sideband.as_ref()
    }

    fn set_sideband(&mut self, sideband: BlockSideband) {
        self.sideband = Some(sideband)
    }

    fn block_type(&self) -> BlockType {
        BlockType::LegacyReceive
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

    fn set_block_signature(&mut self, signature: &Signature) {
        self.signature = signature.clone();
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
        writer.write_bytes_safe(&self.work.to_be_bytes());
    }

    fn serialize_json(&self, writer: &mut dyn PropertyTree) -> Result<()> {
        writer.put_string("type", "receive")?;
        writer.put_string("previous", &self.hashables.previous.encode_hex())?;
        writer.put_string("source", &self.hashables.source.encode_hex())?;
        writer.put_string("work", &to_hex_string(self.work))?;
        writer.put_string("signature", &self.signature.encode_hex())?;
        Ok(())
    }

    fn root(&self) -> Root {
        self.previous().into()
    }

    fn visit(&self, visitor: &mut dyn BlockVisitor) {
        visitor.receive_block(self);
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

    fn visit_mut(&mut self, visitor: &mut dyn super::MutableBlockVisitor) {
        visitor.receive_block(self)
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

#[derive(PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
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
        let hash = LazyBlockHash::new();

        Self {
            work: value.work.into(),
            signature: value.signature,
            hashables,
            hash,
            sideband: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        utils::{MemoryStream, TestPropertyTree},
        BlockEnum, KeyPair,
    };

    #[test]
    fn create_block() {
        let key = KeyPair::new();
        let previous = BlockHash::from(1);
        let block = ReceiveBlock::new(
            previous,
            BlockHash::from(2),
            &key.private_key(),
            &key.public_key(),
            4,
        );
        assert_eq!(block.previous(), previous);
        assert_eq!(block.root(), previous.into());
    }

    // original test: block.receive_serialize
    // original test: receive_block.deserialize
    #[test]
    fn serialize() {
        let key1 = KeyPair::new();
        let block1 = ReceiveBlock::new(
            BlockHash::from(0),
            BlockHash::from(1),
            &key1.private_key(),
            &key1.public_key(),
            4,
        );
        let mut stream = MemoryStream::new();
        block1.serialize_without_block_type(&mut stream);
        assert_eq!(ReceiveBlock::serialized_size(), stream.bytes_written());

        let block2 = ReceiveBlock::deserialize(&mut stream).unwrap();
        assert_eq!(block1, block2);
    }

    // original test: block.receive_serialize_json
    #[test]
    fn serialize_json() {
        let block1 = ReceiveBlock::new_test_instance();
        let mut ptree = TestPropertyTree::new();
        block1.serialize_json(&mut ptree).unwrap();

        let block2 = ReceiveBlock::deserialize_json(&ptree).unwrap();
        assert_eq!(block1, block2);
    }

    #[test]
    fn serialize_serde() {
        let block = BlockEnum::LegacyReceive(ReceiveBlock::new_test_instance());
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
