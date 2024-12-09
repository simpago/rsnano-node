use super::{Block, BlockBase, BlockType};
use crate::{
    utils::{BufferWriter, Deserialize, FixedSizeSerialize, Serialize, Stream},
    Account, Amount, BlockHash, BlockHashBuilder, DependentBlocks, JsonBlock, Link, PrivateKey,
    PublicKey, Root, Signature, WorkNonce,
};
use anyhow::Result;

#[derive(Clone, Debug)]
pub struct OpenBlock {
    work: u64,
    signature: Signature,
    hashables: OpenHashables,
    hash: BlockHash,
}

impl OpenBlock {
    pub fn account(&self) -> Account {
        self.hashables.account
    }

    pub fn new_test_instance() -> Self {
        let key = PrivateKey::from(42);
        OpenBlockArgs {
            key: &key,
            source: BlockHash::from(123),
            representative: PublicKey::from(456),
            work: 69420,
        }
        .into()
    }

    pub fn source(&self) -> BlockHash {
        self.hashables.source
    }

    pub fn representative(&self) -> PublicKey {
        self.hashables.representative
    }

    pub fn serialized_size() -> usize {
        OpenHashables::serialized_size() + Signature::serialized_size() + std::mem::size_of::<u64>()
    }

    pub fn deserialize(stream: &mut dyn Stream) -> Result<Self> {
        let hashables = OpenHashables {
            source: BlockHash::deserialize(stream)?,
            representative: PublicKey::deserialize(stream)?,
            account: Account::deserialize(stream)?,
        };
        let signature = Signature::deserialize(stream)?;
        let mut work_bytes = [0u8; 8];
        stream.read_bytes(&mut work_bytes, 8)?;
        let work = u64::from_le_bytes(work_bytes);
        let hash = hashables.hash();
        Ok(OpenBlock {
            work,
            signature,
            hashables,
            hash,
        })
    }

    pub fn dependent_blocks(&self, genesis_account: &Account) -> DependentBlocks {
        if self.account() == *genesis_account {
            DependentBlocks::none()
        } else {
            DependentBlocks::new(self.source(), BlockHash::zero())
        }
    }
}

impl PartialEq for OpenBlock {
    fn eq(&self, other: &Self) -> bool {
        self.work == other.work
            && self.signature == other.signature
            && self.hashables == other.hashables
    }
}

impl Eq for OpenBlock {}

impl BlockBase for OpenBlock {
    fn block_type(&self) -> BlockType {
        BlockType::LegacyOpen
    }

    fn account_field(&self) -> Option<Account> {
        Some(self.hashables.account)
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
        BlockHash::zero()
    }

    fn serialize_without_block_type(&self, writer: &mut dyn BufferWriter) {
        self.hashables.source.serialize(writer);
        self.hashables.representative.serialize(writer);
        self.hashables.account.serialize(writer);
        self.signature.serialize(writer);
        writer.write_bytes_safe(&self.work.to_le_bytes());
    }

    fn root(&self) -> Root {
        self.hashables.account.into()
    }

    fn balance_field(&self) -> Option<Amount> {
        None
    }

    fn source_field(&self) -> Option<BlockHash> {
        Some(self.hashables.source)
    }

    fn representative_field(&self) -> Option<PublicKey> {
        Some(self.hashables.representative)
    }

    fn valid_predecessor(&self, _block_type: BlockType) -> bool {
        false
    }

    fn qualified_root(&self) -> crate::QualifiedRoot {
        crate::QualifiedRoot::new(self.root(), self.previous())
    }

    fn destination_field(&self) -> Option<Account> {
        None
    }

    fn json_representation(&self) -> JsonBlock {
        JsonBlock::Open(JsonOpenBlock {
            source: self.hashables.source,
            representative: self.hashables.representative.into(),
            account: self.hashables.account,
            work: self.work.into(),
            signature: self.signature.clone(),
        })
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct OpenHashables {
    /// Block with first send transaction to this account
    source: BlockHash,
    representative: PublicKey,
    account: Account,
}

impl OpenHashables {
    fn serialized_size() -> usize {
        BlockHash::serialized_size() + Account::serialized_size() + Account::serialized_size()
    }

    fn hash(&self) -> BlockHash {
        BlockHashBuilder::new()
            .update(self.source.as_bytes())
            .update(self.representative.as_bytes())
            .update(self.account.as_bytes())
            .build()
    }
}

pub struct OpenBlockArgs<'a> {
    pub key: &'a PrivateKey,
    pub source: BlockHash,
    pub representative: PublicKey,
    pub work: u64,
}

impl<'a> From<OpenBlockArgs<'a>> for OpenBlock {
    fn from(value: OpenBlockArgs<'a>) -> Self {
        let hashables = OpenHashables {
            source: value.source,
            representative: value.representative,
            account: value.key.account(),
        };

        let hash = hashables.hash();
        let signature = value.key.sign(hash.as_bytes());

        Self {
            signature,
            hashables,
            hash,
            work: value.work,
        }
    }
}

impl<'a> From<OpenBlockArgs<'a>> for Block {
    fn from(value: OpenBlockArgs<'a>) -> Self {
        Self::LegacyOpen(value.into())
    }
}

#[derive(PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct JsonOpenBlock {
    pub account: Account,
    pub source: BlockHash,
    pub representative: Account,
    pub signature: Signature,
    pub work: WorkNonce,
}

impl From<JsonOpenBlock> for OpenBlock {
    fn from(value: JsonOpenBlock) -> Self {
        let hashables = OpenHashables {
            source: value.source,
            representative: value.representative.into(),
            account: value.account,
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
        let source = BlockHash::from(1);
        let representative = PublicKey::from(2);
        let block: OpenBlock = OpenBlockArgs {
            key: &key,
            source,
            representative,
            work: 0,
        }
        .into();

        assert_eq!(block.account_field(), Some(key.account()));
        assert_eq!(block.root(), key.account().into());
    }

    // original test: open_block.deserialize
    #[test]
    fn serialize() {
        let block1 = OpenBlock::new_test_instance();
        let mut stream = MemoryStream::new();
        block1.serialize_without_block_type(&mut stream);
        assert_eq!(OpenBlock::serialized_size(), stream.bytes_written());

        let block2 = OpenBlock::deserialize(&mut stream).unwrap();
        assert_eq!(block1, block2);
    }

    #[test]
    fn serialize_serde() {
        let block = Block::LegacyOpen(OpenBlock::new_test_instance());
        let serialized = serde_json::to_string_pretty(&block).unwrap();
        assert_eq!(
            serialized,
            r#"{
  "type": "open",
  "account": "nano_39y535msmkzb31bx73tdnf8iken5ucw9jt98re7nriduus6cgs6uonjdm8r5",
  "source": "000000000000000000000000000000000000000000000000000000000000007B",
  "representative": "nano_11111111111111111111111111111111111111111111111111gahteczqci",
  "signature": "A8980EB0E15F4722B4644AF254DC88DF4044ABDFB483DDAC36EDA276122D099105C3EF3B3CD677E6438DEE876B84A9433CFC83CF54F864DE034F7D97A3370C07",
  "work": "0000000000010F2C"
}"#
        );
    }
}
