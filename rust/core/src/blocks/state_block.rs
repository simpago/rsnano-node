use crate::{
    sign_message, to_hex_string, u64_from_hex_str,
    utils::{BufferWriter, Deserialize, FixedSizeSerialize, PropertyTree, Serialize, Stream},
    Account, Amount, BlockHash, BlockHashBuilder, KeyPair, LazyBlockHash, Link, PublicKey, RawKey,
    Root, Signature,
};
use anyhow::Result;
use serde::ser::SerializeStruct;

use super::{Block, BlockSideband, BlockType, BlockVisitor};

#[derive(Clone, PartialEq, Eq, Default, Debug)]
pub struct StateHashables {
    // Account# / public key that operates this account
    // Uses:
    // Bulk signature validation in advance of further ledger processing
    // Arranging uncomitted transactions by account
    pub account: Account,

    // Previous transaction in this chain
    pub previous: BlockHash,

    // Representative of this account
    pub representative: Account,

    // Current balance of this account
    // Allows lookup of account balance simply by looking at the head block
    pub balance: Amount,

    // Link field contains source block_hash if receiving, destination account if sending
    pub link: Link,
}

impl From<&StateHashables> for BlockHash {
    fn from(hashables: &StateHashables) -> Self {
        let mut preamble = [0u8; 32];
        preamble[31] = BlockType::State as u8;
        BlockHashBuilder::new()
            .update(preamble)
            .update(hashables.account.as_bytes())
            .update(hashables.previous.as_bytes())
            .update(hashables.representative.as_bytes())
            .update(hashables.balance.to_be_bytes())
            .update(hashables.link.as_bytes())
            .build()
    }
}

#[derive(Clone, Default, Debug)]
pub struct StateBlock {
    pub work: u64,
    pub signature: Signature,
    pub hashables: StateHashables,
    pub hash: LazyBlockHash,
    pub sideband: Option<BlockSideband>,
}

#[allow(clippy::too_many_arguments)]
impl StateBlock {
    pub fn new(
        account: Account,
        previous: BlockHash,
        representative: Account,
        balance: Amount,
        link: Link,
        keys: &KeyPair,
        work: u64,
    ) -> Self {
        Self::new_obsolete(
            account,
            previous,
            representative,
            balance,
            link,
            &keys.private_key(),
            &keys.public_key(),
            work,
        )
    }

    // Don't use this anymore
    pub fn new_obsolete(
        account: Account,
        previous: BlockHash,
        representative: Account,
        balance: Amount,
        link: Link,
        prv_key: &RawKey,
        pub_key: &PublicKey,
        work: u64,
    ) -> Self {
        let hashables = StateHashables {
            account,
            previous,
            representative,
            balance,
            link,
        };

        let hash = LazyBlockHash::new();
        let signature = sign_message(prv_key, pub_key, hash.hash(&hashables).as_bytes());

        Self {
            work,
            signature,
            hashables,
            hash,
            sideband: None,
        }
    }

    pub fn new_test_instance_with_key(key: KeyPair) -> Self {
        Self::new(
            key.public_key(),
            BlockHash::from(456),
            Account::from(789),
            Amount::raw(420),
            Link::from(111),
            &key,
            69420,
        )
    }

    pub fn new_test_instance() -> Self {
        let key = KeyPair::from(42);
        Self::new_test_instance_with_key(key)
    }

    pub fn with_signature(
        account: Account,
        previous: BlockHash,
        representative: Account,
        balance: Amount,
        link: Link,
        signature: Signature,
        work: u64,
    ) -> Self {
        Self {
            work,
            signature,
            hashables: StateHashables {
                account,
                previous,
                representative,
                balance,
                link,
            },
            hash: LazyBlockHash::new(),
            sideband: None,
        }
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

    pub fn mandatory_representative(&self) -> Account {
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
        let representative = Account::deserialize(stream)?;
        let balance = Amount::deserialize(stream)?;
        let link = Link::deserialize(stream)?;
        let signature = Signature::deserialize(stream)?;
        let mut work_bytes = [0u8; 8];
        stream.read_bytes(&mut work_bytes, 8)?;
        let work = u64::from_be_bytes(work_bytes);
        Ok(Self {
            work,
            signature,
            hashables: StateHashables {
                account,
                previous,
                representative,
                balance,
                link,
            },
            hash: LazyBlockHash::new(),
            sideband: None,
        })
    }

    pub fn deserialize_json(reader: &impl PropertyTree) -> Result<Self> {
        let block_type = reader.get_string("type")?;
        if block_type != "state" {
            bail!("invalid block type");
        }
        let account = Account::decode_account(reader.get_string("account")?)?;
        let previous = BlockHash::decode_hex(reader.get_string("previous")?)?;
        let representative = Account::decode_account(reader.get_string("representative")?)?;
        let balance = Amount::decode_dec(reader.get_string("balance")?)?;
        let link = Link::decode_hex(reader.get_string("link")?)?;
        let work = u64_from_hex_str(reader.get_string("work")?)?;
        let signature = Signature::decode_hex(reader.get_string("signature")?)?;
        Ok(StateBlock {
            work,
            signature,
            hashables: StateHashables {
                account,
                previous,
                representative,
                balance,
                link,
            },
            hash: LazyBlockHash::new(),
            sideband: None,
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

impl Block for StateBlock {
    fn sideband(&'_ self) -> Option<&'_ BlockSideband> {
        self.sideband.as_ref()
    }

    fn set_sideband(&mut self, sideband: BlockSideband) {
        self.sideband = Some(sideband);
    }

    fn block_type(&self) -> BlockType {
        BlockType::State
    }

    fn account_field(&self) -> Option<Account> {
        Some(self.hashables.account)
    }

    fn hash(&self) -> BlockHash {
        self.hash.hash(&self.hashables)
    }

    fn link_field(&self) -> Option<Link> {
        Some(self.hashables.link)
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
        self.hashables.account.serialize(writer);
        self.hashables.previous.serialize(writer);
        self.hashables.representative.serialize(writer);
        self.hashables.balance.serialize(writer);
        self.hashables.link.serialize(writer);
        self.signature.serialize(writer);
        writer.write_bytes_safe(&self.work.to_be_bytes());
    }

    fn serialize_json(&self, writer: &mut dyn PropertyTree) -> Result<()> {
        writer.put_string("type", "state")?;
        writer.put_string("account", &self.hashables.account.encode_account())?;
        writer.put_string("previous", &self.hashables.previous.encode_hex())?;
        writer.put_string(
            "representative",
            &self.hashables.representative.encode_account(),
        )?;
        writer.put_string("balance", &self.hashables.balance.to_string_dec())?;
        writer.put_string("link", &self.hashables.link.encode_hex())?;
        writer.put_string(
            "link_as_account",
            &Account::from(&self.hashables.link).encode_account(),
        )?;
        writer.put_string("signature", &self.signature.encode_hex())?;
        writer.put_string("work", &to_hex_string(self.work))?;
        Ok(())
    }

    fn root(&self) -> Root {
        if !self.previous().is_zero() {
            self.previous().into()
        } else {
            self.hashables.account.into()
        }
    }

    fn visit(&self, visitor: &mut dyn BlockVisitor) {
        visitor.state_block(self)
    }

    fn balance_field(&self) -> Option<Amount> {
        Some(self.hashables.balance)
    }

    fn source_field(&self) -> Option<BlockHash> {
        None
    }

    fn representative_field(&self) -> Option<Account> {
        Some(self.hashables.representative)
    }

    fn visit_mut(&mut self, visitor: &mut dyn super::MutableBlockVisitor) {
        visitor.state_block(self);
    }

    fn valid_predecessor(&self, _block_type: BlockType) -> bool {
        true
    }

    fn destination_field(&self) -> Option<Account> {
        None
    }
}

impl serde::Serialize for StateBlock {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("Block", 9)?;
        state.serialize_field("type", "state")?;
        state.serialize_field("account", &self.hashables.account)?;
        state.serialize_field("previous", &self.hashables.previous)?;
        state.serialize_field("representative", &self.hashables.representative)?;
        state.serialize_field("balance", &self.hashables.balance.to_string_dec())?;
        state.serialize_field("link", &self.hashables.link.encode_hex())?;
        state.serialize_field("link_as_account", &Account::from(&self.hashables.link))?;
        state.serialize_field("signature", &self.signature)?;
        state.serialize_field("work", &to_hex_string(self.work))?;
        state.end()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        utils::{MemoryStream, TestPropertyTree},
        BlockBuilder, BlockEnum, StateBlockBuilder,
    };

    use super::*;

    // original test: state_block.serialization
    #[test]
    fn serialization() {
        let block1 = BlockBuilder::state().work(5).build();
        let mut stream = MemoryStream::new();
        block1.serialize_without_block_type(&mut stream);
        assert_eq!(StateBlock::serialized_size(), stream.bytes_written());
        assert_eq!(stream.byte_at(215), 0x5); // Ensure work is serialized big-endian

        let block2 = StateBlock::deserialize(&mut stream).unwrap();
        assert_eq!(block1, BlockEnum::State(block2));
    }

    // original test: state_block.serialization
    #[test]
    fn json_serialization() {
        let block1 = BlockBuilder::state().build();

        let mut ptree = TestPropertyTree::new();
        block1.serialize_json(&mut ptree).unwrap();

        let block2 = StateBlock::deserialize_json(&ptree).unwrap();
        assert_eq!(block1, BlockEnum::State(block2));
    }

    // original test: state_block.hashing
    #[test]
    fn hashing() {
        let block = BlockBuilder::state().build();
        let hash = block.hash().clone();
        assert_eq!(hash, block.hash()); // check cache works
        assert_eq!(hash, BlockBuilder::state().build().hash());

        let assert_different_hash = |b: StateBlockBuilder| {
            assert_ne!(hash, b.build().hash());
        };

        assert_different_hash(BlockBuilder::state().account(Account::from(1000)));
        assert_different_hash(BlockBuilder::state().previous(BlockHash::from(1000)));
        assert_different_hash(BlockBuilder::state().representative(Account::from(1000)));
        assert_different_hash(BlockBuilder::state().balance(Amount::from(1000)));
        assert_different_hash(BlockBuilder::state().link(Link::from(1000)));
    }

    #[test]
    fn serialize_serde() {
        let block = StateBlock::new_test_instance();
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
