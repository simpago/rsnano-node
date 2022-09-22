use crate::{
    from_string_hex, sign_message, to_string_hex,
    utils::{Deserialize, PropertyTreeReader, PropertyTreeWriter, Serialize, Stream},
    Account, Block, BlockHash, BlockHashBuilder, BlockSideband, BlockType, LazyBlockHash, Link,
    PublicKey, RawKey, Root, Signature,
};
use anyhow::Result;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct OpenHashables {
    pub source: BlockHash,
    pub representative: Account,
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
    pub sideband: Option<BlockSideband>,
}

impl OpenBlock {
    pub fn new(
        source: BlockHash,
        representative: Account,
        account: Account,
        prv_key: &RawKey,
        pub_key: &PublicKey,
        work: u64,
    ) -> Result<Self> {
        let hashables = OpenHashables {
            source,
            representative,
            account,
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

    pub fn serialized_size() -> usize {
        OpenHashables::serialized_size() + Signature::serialized_size() + std::mem::size_of::<u64>()
    }

    pub fn deserialize(stream: &mut dyn Stream) -> Result<Self> {
        let hashables = OpenHashables {
            source: BlockHash::deserialize(stream)?,
            representative: Account::deserialize(stream)?,
            account: Account::deserialize(stream)?,
        };
        let signature = Signature::deserialize(stream)?;
        let mut work_bytes = [0u8; 8];
        stream.read_bytes(&mut work_bytes, 8)?;
        let work = u64::from_be_bytes(work_bytes);
        Ok(OpenBlock {
            work,
            signature,
            hashables,
            hash: LazyBlockHash::new(),
            sideband: None,
        })
    }

    pub fn deserialize_json(reader: &impl PropertyTreeReader) -> Result<Self> {
        let source = BlockHash::decode_hex(reader.get_string("source")?)?;
        let representative = Account::decode_account(reader.get_string("representative")?)?;
        let account = Account::decode_account(reader.get_string("account")?)?;
        let work = from_string_hex(reader.get_string("work")?)?;
        let signature = Signature::decode_hex(reader.get_string("signature")?)?;
        Ok(OpenBlock {
            work,
            signature,
            hashables: OpenHashables {
                source,
                representative,
                account,
            },
            hash: LazyBlockHash::new(),
            sideband: None,
        })
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

impl Block for OpenBlock {
    fn sideband(&'_ self) -> Option<&'_ BlockSideband> {
        self.sideband.as_ref()
    }

    fn set_sideband(&mut self, sideband: BlockSideband) {
        self.sideband = Some(sideband);
    }

    fn block_type(&self) -> BlockType {
        BlockType::Open
    }

    fn account(&self) -> &Account {
        &self.hashables.account
    }

    fn hash(&self) -> BlockHash {
        self.hash.hash(&self.hashables)
    }

    fn link(&self) -> crate::Link {
        Link::new()
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
        BlockHash::zero()
    }

    fn serialize(&self, stream: &mut dyn Stream) -> Result<()> {
        self.hashables.source.serialize(stream)?;
        self.hashables.representative.serialize(stream)?;
        self.hashables.account.serialize(stream)?;
        self.signature.serialize(stream)?;
        stream.write_bytes(&self.work.to_be_bytes())?;
        Ok(())
    }

    fn serialize_json(&self, writer: &mut dyn PropertyTreeWriter) -> Result<()> {
        writer.put_string("type", "open")?;
        writer.put_string("source", &self.hashables.source.encode_hex())?;
        writer.put_string(
            "representative",
            &self.hashables.representative.encode_account(),
        )?;
        writer.put_string("account", &self.hashables.account.encode_account())?;
        writer.put_string("work", &to_string_hex(self.work))?;
        writer.put_string("signature", &self.signature.encode_hex())?;
        Ok(())
    }

    fn root(&self) -> Root {
        self.account().into()
    }

    fn visit(&self, visitor: &mut dyn crate::BlockVisitor) {
        visitor.open_block(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        numbers::KeyPair,
        utils::{MemoryStream, TestPropertyTree},
    };

    // original test: block.open_serialize_json
    #[test]
    fn serialize_json() -> Result<()> {
        let key1 = KeyPair::new();
        let block1 = OpenBlock::new(
            BlockHash::from(0),
            Account::from(1),
            Account::from(0),
            &key1.private_key(),
            &key1.public_key(),
            0,
        )?;
        let mut ptree = TestPropertyTree::new();
        block1.serialize_json(&mut ptree)?;

        let block2 = OpenBlock::deserialize_json(&ptree)?;
        assert_eq!(block1, block2);
        Ok(())
    }

    // original test: open_block.deserialize
    #[test]
    fn serialize() -> Result<()> {
        let key1 = KeyPair::new();
        let block1 = OpenBlock::new(
            BlockHash::from(0),
            Account::from(1),
            Account::from(0),
            &key1.private_key(),
            &key1.public_key(),
            0,
        )?;
        let mut stream = MemoryStream::new();
        block1.serialize(&mut stream)?;
        assert_eq!(OpenBlock::serialized_size(), stream.bytes_written());

        let block2 = OpenBlock::deserialize(&mut stream)?;
        assert_eq!(block1, block2);
        Ok(())
    }
}
