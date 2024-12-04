use super::{BlockBase, BlockType};
use crate::{
    utils::{BufferWriter, Deserialize, FixedSizeSerialize, Serialize, Stream},
    Account, Amount, BlockHash, BlockHashBuilder, DependentBlocks, JsonBlock, LazyBlockHash, Link,
    PrivateKey, PublicKey, Root, Signature, WorkNonce,
};
use anyhow::Result;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct OpenHashables {
    /// Block with first send transaction to this account
    pub source: BlockHash,
    pub representative: PublicKey,
    pub account: Account,
}

impl OpenHashables {
    fn serialized_size() -> usize {
        BlockHash::serialized_size() + Account::serialized_size() + Account::serialized_size()
    }
}

impl From<&OpenHashables> for BlockHash {
    fn from(hashables: &OpenHashables) -> Self {
        BlockHashBuilder::new()
            .update(hashables.source.as_bytes())
            .update(hashables.representative.as_bytes())
            .update(hashables.account.as_bytes())
            .build()
    }
}

#[derive(Clone, Debug)]
pub struct OpenBlock {
    pub work: u64,
    pub signature: Signature,
    pub hashables: OpenHashables,
    pub hash: LazyBlockHash,
}

impl OpenBlock {
    pub fn new(
        source: BlockHash,
        representative: PublicKey,
        account: Account,
        prv_key: &PrivateKey,
        work: u64,
    ) -> Self {
        let hashables = OpenHashables {
            source,
            representative,
            account,
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

    pub fn account(&self) -> Account {
        self.hashables.account
    }

    pub fn new_test_instance() -> Self {
        let key = PrivateKey::from(42);
        Self::new(
            BlockHash::from(123),
            PublicKey::from(456),
            Account::from(789),
            &key,
            69420,
        )
    }

    pub fn source(&self) -> BlockHash {
        self.hashables.source
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
        Ok(OpenBlock {
            work,
            signature,
            hashables,
            hash: LazyBlockHash::new(),
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
    use crate::{utils::MemoryStream, Block, PrivateKey};

    #[test]
    fn create_block() {
        let key = PrivateKey::new();
        let source = BlockHash::from(1);
        let representative = PublicKey::from(2);
        let account = Account::from(3);
        let block = OpenBlock::new(source, representative, account, &key, 0);

        assert_eq!(block.account_field(), Some(account));
        assert_eq!(block.root(), account.into());
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
  "account": "nano_11111111111111111111111111111111111111111111111111ros3kc7wyy",
  "source": "000000000000000000000000000000000000000000000000000000000000007B",
  "representative": "nano_11111111111111111111111111111111111111111111111111gahteczqci",
  "signature": "791B637D0CB7D333AFC9F4D06870A1B5ADD2857E5C37BBAEEF70C77E0DDC7DF6541CC877EA88BE2483D7E0198BC9455C61E4B7BD98A50352BB5C4AD0E468DF04",
  "work": "0000000000010F2C"
}"#
        );
    }
}
