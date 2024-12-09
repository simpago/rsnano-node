use super::{Block, BlockBase, BlockType};
use crate::{
    utils::{BufferWriter, Deserialize, FixedSizeSerialize, Serialize, Stream},
    Account, Amount, BlockHash, BlockHashBuilder, JsonBlock, Link, PrivateKey, PublicKey, Root,
    Signature, WorkNonce,
};
use anyhow::Result;

#[derive(Clone, Default, Debug)]
pub struct StateBlock {
    hashables: StateHashables,
    signature: Signature,
    hash: BlockHash,
    work: u64,
}

impl StateBlock {
    pub fn verify_signature(&self) -> anyhow::Result<()> {
        self.account()
            .as_key()
            .verify(self.hash().as_bytes(), self.signature())
    }

    pub fn account(&self) -> Account {
        self.hashables.account
    }

    pub fn link(&self) -> Link {
        self.hashables.link
    }

    pub fn balance(&self) -> Amount {
        self.hashables.balance
    }

    pub fn source(&self) -> BlockHash {
        BlockHash::zero()
    }

    pub fn representative(&self) -> PublicKey {
        self.hashables.representative
    }

    pub fn destination(&self) -> Account {
        Account::zero()
    }

    pub fn serialized_size() -> usize {
        Account::serialized_size() // Account
            + BlockHash::serialized_size() // Previous
            + Account::serialized_size() // Representative
            + Amount::serialized_size() // Balance
            + Link::serialized_size() // Link
            + Signature::serialized_size()
            + std::mem::size_of::<u64>() // Work
    }

    pub fn deserialize(stream: &mut dyn Stream) -> Result<Self> {
        let account = Account::deserialize(stream)?;
        let previous = BlockHash::deserialize(stream)?;
        let representative = PublicKey::deserialize(stream)?;
        let balance = Amount::deserialize(stream)?;
        let link = Link::deserialize(stream)?;
        let signature = Signature::deserialize(stream)?;
        let mut work_bytes = [0u8; 8];
        stream.read_bytes(&mut work_bytes, 8)?;
        let work = u64::from_be_bytes(work_bytes);
        let hashables = StateHashables {
            account,
            previous,
            representative,
            balance,
            link,
        };
        let hash = hashables.hash();
        Ok(Self {
            work,
            signature,
            hashables,
            hash,
        })
    }
}

impl PartialEq for StateBlock {
    fn eq(&self, other: &Self) -> bool {
        self.work == other.work
            && self.signature == other.signature
            && self.hashables == other.hashables
    }
}

impl Eq for StateBlock {}

impl BlockBase for StateBlock {
    fn block_type(&self) -> BlockType {
        BlockType::State
    }

    fn account_field(&self) -> Option<Account> {
        Some(self.hashables.account)
    }

    fn hash(&self) -> BlockHash {
        self.hash
    }

    fn link_field(&self) -> Option<Link> {
        Some(self.hashables.link)
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
        self.hashables.account.serialize(writer);
        self.hashables.previous.serialize(writer);
        self.hashables.representative.serialize(writer);
        self.hashables.balance.serialize(writer);
        self.hashables.link.serialize(writer);
        self.signature.serialize(writer);
        writer.write_bytes_safe(&self.work.to_be_bytes());
    }

    fn root(&self) -> Root {
        if !self.previous().is_zero() {
            self.previous().into()
        } else {
            self.hashables.account.into()
        }
    }

    fn balance_field(&self) -> Option<Amount> {
        Some(self.hashables.balance)
    }

    fn source_field(&self) -> Option<BlockHash> {
        None
    }

    fn representative_field(&self) -> Option<PublicKey> {
        Some(self.hashables.representative)
    }

    fn valid_predecessor(&self, _block_type: BlockType) -> bool {
        true
    }

    fn destination_field(&self) -> Option<Account> {
        None
    }

    fn json_representation(&self) -> JsonBlock {
        JsonBlock::State(JsonStateBlock {
            account: self.hashables.account,
            previous: self.hashables.previous,
            representative: self.hashables.representative.into(),
            balance: self.hashables.balance,
            link: self.hashables.link,
            link_as_account: Some(self.hashables.link.into()),
            signature: self.signature.clone(),
            work: self.work.into(),
        })
    }
}

#[derive(Clone, PartialEq, Eq, Default, Debug)]
struct StateHashables {
    // Account# / public key that operates this account
    // Uses:
    // Bulk signature validation in advance of further ledger processing
    // Arranging uncomitted transactions by account
    account: Account,

    // Previous transaction in this chain
    previous: BlockHash,

    // Representative of this account
    representative: PublicKey,

    // Current balance of this account
    // Allows lookup of account balance simply by looking at the head block
    balance: Amount,

    // Link field contains source block_hash if receiving, destination account if sending
    link: Link,
}

impl StateHashables {
    fn hash(&self) -> BlockHash {
        let mut preamble = [0u8; 32];
        preamble[31] = BlockType::State as u8;
        BlockHashBuilder::new()
            .update(preamble)
            .update(self.account.as_bytes())
            .update(self.previous.as_bytes())
            .update(self.representative.as_bytes())
            .update(self.balance.to_be_bytes())
            .update(self.link.as_bytes())
            .build()
    }
}

#[derive(Clone)]
pub struct StateBlockArgs<'a> {
    pub key: &'a PrivateKey,
    pub previous: BlockHash,
    pub representative: PublicKey,
    pub balance: Amount,
    pub link: Link,
    pub work: u64,
}

impl<'a> From<StateBlockArgs<'a>> for Block {
    fn from(value: StateBlockArgs<'a>) -> Self {
        let hashables = StateHashables {
            account: value.key.account(),
            previous: value.previous,
            representative: value.representative,
            balance: value.balance,
            link: value.link,
        };

        let hash = hashables.hash();
        let signature = value.key.sign(hash.as_bytes());

        Block::State(StateBlock {
            hashables,
            signature,
            hash,
            work: value.work,
        })
    }
}

pub struct EpochBlockArgs<'a> {
    pub epoch_signer: &'a PrivateKey,
    pub account: Account,
    pub previous: BlockHash,
    pub representative: PublicKey,
    pub balance: Amount,
    pub link: Link,
    pub work: u64,
}

impl<'a> From<EpochBlockArgs<'a>> for Block {
    fn from(value: EpochBlockArgs<'a>) -> Self {
        let hashables = StateHashables {
            account: value.account,
            previous: value.previous,
            representative: value.representative,
            balance: value.balance,
            link: value.link,
        };

        let hash = hashables.hash();
        let signature = value.epoch_signer.sign(hash.as_bytes());

        Block::State(StateBlock {
            hashables,
            signature,
            hash,
            work: value.work,
        })
    }
}

impl From<JsonStateBlock> for StateBlock {
    fn from(value: JsonStateBlock) -> Self {
        let hashables = StateHashables {
            account: value.account,
            previous: value.previous,
            representative: value.representative.into(),
            balance: value.balance,
            link: value.link,
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

#[derive(PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct JsonStateBlock {
    pub account: Account,
    pub previous: BlockHash,
    pub representative: Account,
    pub balance: Amount,
    pub link: Link,
    pub link_as_account: Option<Account>,
    pub signature: Signature,
    pub work: WorkNonce,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{utils::MemoryStream, Block, TestBlockBuilder, TestStateBlockBuilder};

    #[test]
    fn serialization() {
        let block1 = TestBlockBuilder::state().work(5).build();
        let mut stream = MemoryStream::new();
        block1.serialize_without_block_type(&mut stream);
        assert_eq!(StateBlock::serialized_size(), stream.bytes_written());
        assert_eq!(stream.byte_at(215), 0x5); // Ensure work is serialized big-endian

        let block2 = StateBlock::deserialize(&mut stream).unwrap();
        assert_eq!(block1, Block::State(block2));
    }

    #[test]
    fn hashing() {
        let key = PrivateKey::from(42);
        let block = TestBlockBuilder::state().key(&key).build();
        let hash = block.hash();
        assert_eq!(hash, TestBlockBuilder::state().key(&key).build().hash());

        let assert_different_hash = |b: TestStateBlockBuilder| {
            assert_ne!(hash, b.build().hash());
        };

        assert_different_hash(
            TestBlockBuilder::state()
                .key(&key)
                .account(Account::from(1000)),
        );
        assert_different_hash(
            TestBlockBuilder::state()
                .key(&key)
                .previous(BlockHash::from(1000)),
        );
        assert_different_hash(
            TestBlockBuilder::state()
                .key(&key)
                .representative(Account::from(1000)),
        );
        assert_different_hash(
            TestBlockBuilder::state()
                .key(&key)
                .balance(Amount::from(1000)),
        );
        assert_different_hash(TestBlockBuilder::state().key(&key).link(Link::from(1000)));
    }

    #[test]
    fn serialize_serde() {
        let block = Block::new_test_instance();
        let serialized = serde_json::to_string_pretty(&block).unwrap();
        assert_eq!(
            serialized,
            r#"{
  "type": "state",
  "account": "nano_39y535msmkzb31bx73tdnf8iken5ucw9jt98re7nriduus6cgs6uonjdm8r5",
  "previous": "00000000000000000000000000000000000000000000000000000000000001C8",
  "representative": "nano_11111111111111111111111111111111111111111111111111ros3kc7wyy",
  "balance": "420",
  "link": "000000000000000000000000000000000000000000000000000000000000006F",
  "link_as_account": "nano_111111111111111111111111111111111111111111111111115hkrzwewgm",
  "signature": "F26EC6180795C63CFEC46F929DCF6269445208B6C1C837FA64925F1D61C218D4D263F9A73A4B76E3174888C6B842FC1380AC15183FA67E92B2091FEBCCBDB308",
  "work": "0000000000010F2C"
}"#
        );
    }
}
