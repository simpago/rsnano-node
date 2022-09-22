use crate::{
    from_string_hex, sign_message, to_string_hex,
    utils::{Deserialize, PropertyTreeReader, PropertyTreeWriter, Serialize, Stream},
    Account, Amount, Block, BlockHash, BlockHashBuilder, BlockSideband, BlockType, LazyBlockHash,
    Link, PublicKey, RawKey, Root, Signature,
};

use anyhow::Result;

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
            .update(&preamble)
            .update(hashables.account.as_bytes())
            .update(hashables.previous.as_bytes())
            .update(hashables.representative.as_bytes())
            .update(&hashables.balance.to_be_bytes())
            .update(&hashables.link.to_bytes())
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
        prv_key: &RawKey,
        pub_key: &PublicKey,
        work: u64,
    ) -> Result<Self> {
        let hashables = StateHashables {
            account,
            previous,
            representative,
            balance,
            link,
        };

        let hash = LazyBlockHash::new();
        let signature = sign_message(prv_key, pub_key, hash.hash(&hashables).as_bytes())?;

        Ok(Self {
            work,
            signature,
            hashables,
            hash,
            sideband: None,
        })
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

    pub fn source(&'_ self) -> &'_ BlockHash {
        BlockHash::zero()
    }

    pub fn destination(&'_ self) -> &'_ Account {
        Account::zero()
    }

    pub fn balance(&self) -> Amount {
        self.hashables.balance
    }

    fn sign(&mut self, prv_key: &RawKey, pub_key: &PublicKey) -> Result<()> {
        let signature = sign_message(prv_key, pub_key, self.hash().as_bytes())?;
        self.signature = signature;
        Ok(())
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

    pub fn deserialize_json(reader: &impl PropertyTreeReader) -> Result<Self> {
        let block_type = reader.get_string("type")?;
        if block_type != "state" {
            bail!("invalid block type");
        }
        let account = Account::decode_account(reader.get_string("account")?)?;
        let previous = BlockHash::decode_hex(reader.get_string("previous")?)?;
        let representative = Account::decode_account(reader.get_string("representative")?)?;
        let balance = Amount::decode_dec(reader.get_string("balance")?)?;
        let link = Link::decode_hex(reader.get_string("link")?)?;
        let work = from_string_hex(reader.get_string("work")?)?;
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

    fn account(&self) -> &Account {
        &self.hashables.account
    }

    fn hash(&self) -> BlockHash {
        self.hash.hash(&self.hashables)
    }

    fn link(&self) -> Link {
        self.hashables.link
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

    fn previous(&self) -> &BlockHash {
        &self.hashables.previous
    }

    fn serialize(&self, stream: &mut dyn Stream) -> Result<()> {
        self.hashables.account.serialize(stream)?;
        self.hashables.previous.serialize(stream)?;
        self.hashables.representative.serialize(stream)?;
        self.hashables.balance.serialize(stream)?;
        self.hashables.link.serialize(stream)?;
        self.signature.serialize(stream)?;
        stream.write_bytes(&self.work.to_be_bytes())?;
        Ok(())
    }

    fn serialize_json(&self, writer: &mut dyn PropertyTreeWriter) -> Result<()> {
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
            &self.hashables.link.to_account().encode_account(),
        )?;
        writer.put_string("signature", &self.signature.encode_hex())?;
        writer.put_string("work", &to_string_hex(self.work))?;
        Ok(())
    }

    fn root(&self) -> Root {
        if !self.previous().is_zero() {
            self.previous().into()
        } else {
            self.account().into()
        }
    }

    fn visit(&self, visitor: &mut dyn crate::BlockVisitor) {
        visitor.state_block(self)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        blocks::{BlockBuilder, StateBlockBuilder},
        utils::{MemoryStream, TestPropertyTree},
    };

    use super::*;

    // original test: state_block.serialization
    #[test]
    fn serialization() -> Result<()> {
        let block1 = BlockBuilder::state().work(5).build()?;
        let mut stream = MemoryStream::new();
        block1.serialize(&mut stream)?;
        assert_eq!(StateBlock::serialized_size(), stream.bytes_written());
        assert_eq!(stream.byte_at(215), 0x5); // Ensure work is serialized big-endian

        let block2 = StateBlock::deserialize(&mut stream)?;
        assert_eq!(block1, block2);

        Ok(())
    }

    // original test: state_block.serialization
    #[test]
    fn json_serialization() -> Result<()> {
        let block1 = BlockBuilder::state().build()?;

        let mut ptree = TestPropertyTree::new();
        block1.serialize_json(&mut ptree)?;

        let block2 = StateBlock::deserialize_json(&ptree)?;
        assert_eq!(block1, block2);

        Ok(())
    }

    // original test: state_block.hashing
    #[test]
    fn hashing() {
        let block = BlockBuilder::state().build().unwrap();
        let hash = block.hash().clone();
        assert_eq!(hash, block.hash()); // check cache works
        assert_eq!(hash, BlockBuilder::state().build().unwrap().hash());

        let assert_different_hash = |b: StateBlockBuilder| {
            assert_ne!(hash, b.build().unwrap().hash());
        };

        assert_different_hash(BlockBuilder::state().account(Account::from(1000)));
        assert_different_hash(BlockBuilder::state().previous(BlockHash::from(1000)));
        assert_different_hash(BlockBuilder::state().representative(Account::from(1000)));
        assert_different_hash(BlockBuilder::state().balance(Amount::from(1000)));
        assert_different_hash(BlockBuilder::state().link(Link::from(1000)));
    }
}
