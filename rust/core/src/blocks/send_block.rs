use super::{Block, BlockSideband, BlockType, BlockVisitor};
use crate::{
    sign_message, to_hex_string, u64_from_hex_str,
    utils::{BufferWriter, FixedSizeSerialize, PropertyTree, Serialize, Stream},
    Account, Amount, BlockHash, BlockHashBuilder, JsonBlock, KeyPair, LazyBlockHash, Link,
    PendingKey, PublicKey, RawKey, Root, Signature, WorkNonce,
};
use anyhow::Result;
use serde::de::{Unexpected, Visitor};

#[derive(Clone, PartialEq, Eq, Default, Debug)]
pub struct SendHashables {
    pub previous: BlockHash,
    pub destination: Account,
    pub balance: Amount,
}

impl SendHashables {
    pub fn serialized_size() -> usize {
        BlockHash::serialized_size() + Account::serialized_size() + Amount::serialized_size()
    }

    pub fn deserialize(stream: &mut dyn Stream) -> Result<Self> {
        let mut buffer_32 = [0u8; 32];
        let mut buffer_16 = [0u8; 16];

        stream.read_bytes(&mut buffer_32, 32)?;
        let previous = BlockHash::from_bytes(buffer_32);

        stream.read_bytes(&mut buffer_32, 32)?;
        let destination = Account::from_bytes(buffer_32);

        stream.read_bytes(&mut buffer_16, 16)?;
        let balance = Amount::raw(u128::from_be_bytes(buffer_16));

        Ok(Self {
            previous,
            destination,
            balance,
        })
    }

    fn clear(&mut self) {
        self.previous = BlockHash::zero();
        self.destination = Account::zero();
        self.balance = Amount::raw(0);
    }
}

impl crate::utils::Serialize for SendHashables {
    fn serialize(&self, stream: &mut dyn BufferWriter) {
        self.previous.serialize(stream);
        self.destination.serialize(stream);
        self.balance.serialize(stream);
    }
}

impl From<&SendHashables> for BlockHash {
    fn from(hashables: &SendHashables) -> Self {
        BlockHashBuilder::new()
            .update(hashables.previous.as_bytes())
            .update(hashables.destination.as_bytes())
            .update(hashables.balance.to_be_bytes())
            .build()
    }
}

#[derive(Clone, Default, Debug)]
pub struct SendBlock {
    pub hashables: SendHashables,
    pub signature: Signature,
    pub work: u64,
    pub hash: LazyBlockHash,
    pub sideband: Option<BlockSideband>,
}

impl SendBlock {
    pub fn new(
        previous: &BlockHash,
        destination: &Account,
        balance: &Amount,
        private_key: &RawKey,
        public_key: &PublicKey,
        work: u64,
    ) -> Self {
        let hashables = SendHashables {
            previous: *previous,
            destination: *destination,
            balance: *balance,
        };

        let hash = LazyBlockHash::new();
        let signature = sign_message(private_key, hash.hash(&hashables).as_bytes());

        Self {
            hashables,
            work,
            signature,
            hash,
            sideband: None,
        }
    }

    pub fn new_test_instance() -> Self {
        let key = KeyPair::from(42);
        SendBlock::new(
            &BlockHash::from(1),
            &Account::from(2),
            &Amount::raw(3),
            &key.private_key(),
            &key.public_key(),
            424269420,
        )
    }

    pub fn deserialize(stream: &mut dyn Stream) -> Result<Self> {
        let hashables = SendHashables::deserialize(stream)?;
        let signature = Signature::deserialize(stream)?;

        let mut buffer = [0u8; 8];
        stream.read_bytes(&mut buffer, 8)?;
        let work = u64::from_le_bytes(buffer);
        Ok(SendBlock {
            hashables,
            signature,
            work,
            hash: LazyBlockHash::new(),
            sideband: None,
        })
    }

    pub fn serialized_size() -> usize {
        SendHashables::serialized_size() + Signature::serialized_size() + std::mem::size_of::<u64>()
    }

    pub fn zero(&mut self) {
        self.work = 0;
        self.signature = Signature::new();
        self.hashables.clear();
    }

    pub fn balance(&self) -> Amount {
        self.hashables.balance
    }

    pub fn set_destination(&mut self, destination: Account) {
        self.hashables.destination = destination;
    }

    pub fn set_previous(&mut self, previous: BlockHash) {
        self.hashables.previous = previous;
    }

    pub fn set_balance(&mut self, balance: Amount) {
        self.hashables.balance = balance;
    }

    pub fn deserialize_json(reader: &impl PropertyTree) -> Result<Self> {
        let previous = BlockHash::decode_hex(reader.get_string("previous")?)?;
        let destination = Account::decode_account(reader.get_string("destination")?)?;
        let balance = Amount::decode_hex(reader.get_string("balance")?)?;
        let signature = Signature::decode_hex(reader.get_string("signature")?)?;
        let work = u64_from_hex_str(reader.get_string("work")?)?;
        Ok(SendBlock {
            hashables: SendHashables {
                previous,
                destination,
                balance,
            },
            signature,
            work,
            hash: LazyBlockHash::new(),
            sideband: None,
        })
    }

    pub fn pending_key(&self) -> PendingKey {
        PendingKey::new(self.hashables.destination, self.hash())
    }

    pub fn destination(&self) -> &Account {
        &self.hashables.destination
    }
}

pub fn valid_send_block_predecessor(block_type: BlockType) -> bool {
    match block_type {
        BlockType::LegacySend
        | BlockType::LegacyReceive
        | BlockType::LegacyOpen
        | BlockType::LegacyChange => true,
        BlockType::NotABlock | BlockType::State | BlockType::Invalid => false,
    }
}

impl PartialEq for SendBlock {
    fn eq(&self, other: &Self) -> bool {
        self.hashables == other.hashables
            && self.signature == other.signature
            && self.work == other.work
    }
}

impl Eq for SendBlock {}

impl Block for SendBlock {
    fn sideband(&'_ self) -> Option<&'_ BlockSideband> {
        self.sideband.as_ref()
    }

    fn set_sideband(&mut self, sideband: BlockSideband) {
        self.sideband = Some(sideband);
    }

    fn block_type(&self) -> BlockType {
        BlockType::LegacySend
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
        self.hashables.serialize(writer);
        self.signature.serialize(writer);
        writer.write_bytes_safe(&self.work.to_le_bytes());
    }

    fn serialize_json(&self, writer: &mut dyn PropertyTree) -> Result<()> {
        writer.put_string("type", "send")?;
        writer.put_string("previous", &self.hashables.previous.encode_hex())?;
        writer.put_string("destination", &self.hashables.destination.encode_account())?;
        writer.put_string("balance", &self.hashables.balance.encode_hex())?;
        writer.put_string("work", &to_hex_string(self.work))?;
        writer.put_string("signature", &self.signature.encode_hex())?;
        Ok(())
    }

    fn root(&self) -> Root {
        self.previous().into()
    }

    fn visit(&self, visitor: &mut dyn BlockVisitor) {
        visitor.send_block(self);
    }

    fn balance_field(&self) -> Option<Amount> {
        Some(self.hashables.balance)
    }

    fn source_field(&self) -> Option<BlockHash> {
        None
    }

    fn representative_field(&self) -> Option<PublicKey> {
        None
    }

    fn visit_mut(&mut self, visitor: &mut dyn super::MutableBlockVisitor) {
        visitor.send_block(self);
    }

    fn valid_predecessor(&self, block_type: BlockType) -> bool {
        valid_send_block_predecessor(block_type)
    }

    fn destination_field(&self) -> Option<Account> {
        Some(self.hashables.destination)
    }

    fn json_representation(&self) -> JsonBlock {
        JsonBlock::Send(JsonSendBlock {
            previous: self.hashables.previous,
            destination: self.hashables.destination,
            balance: self.hashables.balance.into(),
            work: self.work.into(),
            signature: self.signature.clone(),
        })
    }
}

impl From<JsonSendBlock> for SendBlock {
    fn from(value: JsonSendBlock) -> Self {
        let hashables = SendHashables {
            previous: value.previous,
            destination: value.destination,
            balance: value.balance.into(),
        };

        let hash = LazyBlockHash::new();

        Self {
            hashables,
            work: value.work.into(),
            signature: value.signature,
            hash,
            sideband: None,
        }
    }
}

#[derive(PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct JsonSendBlock {
    pub previous: BlockHash,
    pub destination: Account,
    pub balance: AmountHex,
    pub work: WorkNonce,
    pub signature: Signature,
}

#[derive(Debug, PartialEq, Eq)]
pub struct AmountHex(u128);

impl AmountHex {
    pub fn new(amount: u128) -> Self {
        Self(amount)
    }
}

impl From<Amount> for AmountHex {
    fn from(value: Amount) -> Self {
        Self(value.number())
    }
}

impl From<AmountHex> for Amount {
    fn from(value: AmountHex) -> Self {
        Amount::raw(value.0)
    }
}

impl serde::Serialize for AmountHex {
    fn serialize<S>(&self, serializer: S) -> std::prelude::v1::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let amount = Amount::raw(self.0);
        let hex = amount.encode_hex();
        serializer.serialize_str(&hex)
    }
}

impl<'de> serde::Deserialize<'de> for AmountHex {
    fn deserialize<D>(deserializer: D) -> std::prelude::v1::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = deserializer.deserialize_str(AmountHexVisitor {})?;
        Ok(value)
    }
}

struct AmountHexVisitor {}

impl<'de> Visitor<'de> for AmountHexVisitor {
    type Value = AmountHex;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a u128 bit amount in encoded as hex string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let amount = Amount::decode_hex(v).map_err(|_| {
            serde::de::Error::invalid_value(
                Unexpected::Str(v),
                &"a u128 bit amount in encoded as hex string",
            )
        })?;
        Ok(amount.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        utils::{MemoryStream, TestPropertyTree},
        validate_message, BlockEnum, KeyPair,
    };

    #[test]
    fn create_send_block() {
        let key = KeyPair::new();
        let mut block = SendBlock::new(
            &BlockHash::from(0),
            &Account::from(1),
            &Amount::raw(13),
            &key.private_key(),
            &key.public_key(),
            2,
        );

        assert_eq!(block.root(), block.previous().into());
        let hash = block.hash().to_owned();
        assert!(validate_message(&key.public_key(), hash.as_bytes(), &block.signature).is_ok());

        block.signature.make_invalid();
        assert!(validate_message(&key.public_key(), hash.as_bytes(), &block.signature).is_err());
    }

    // original test: block.send_serialize
    // original test: send_block.deserialize
    #[test]
    fn serialize() {
        let key = KeyPair::new();
        let block1 = SendBlock::new(
            &BlockHash::from(0),
            &Account::from(1),
            &Amount::raw(2),
            &key.private_key(),
            &key.public_key(),
            5,
        );
        let mut stream = MemoryStream::new();
        block1.serialize_without_block_type(&mut stream);
        assert_eq!(SendBlock::serialized_size(), stream.bytes_written());

        let block2 = SendBlock::deserialize(&mut stream).unwrap();
        assert_eq!(block1, block2);
    }

    // originial test: block.send_serialize_json
    #[test]
    fn serialize_json() {
        let block1 = SendBlock::new_test_instance();

        let mut ptree = TestPropertyTree::new();
        block1.serialize_json(&mut ptree).unwrap();

        let block2 = SendBlock::deserialize_json(&ptree).unwrap();
        assert_eq!(block1, block2);
    }

    #[test]
    fn serialize_serde() {
        let block = BlockEnum::LegacySend(SendBlock::new_test_instance());
        let serialized = serde_json::to_string_pretty(&block).unwrap();
        assert_eq!(
            serialized,
            r#"{
  "type": "send",
  "previous": "0000000000000000000000000000000000000000000000000000000000000001",
  "destination": "nano_11111111111111111111111111111111111111111111111111147dcwzp3c",
  "balance": "00000000000000000000000000000003",
  "work": "000000001949D66C",
  "signature": "076FF9D1587141EC1DDB05493092B0BFE160B6EEE96D37462B11A81F2622A5211756316A9B48BB403EE4AC57BCCA2023C2075F7214B6B33211B9E5350B76A606"
}"#
        );
    }

    #[test]
    fn serde_serialize_amount_hex() {
        let serialized =
            serde_json::to_string_pretty(&AmountHex::new(337010421085160209006996005437231978653))
                .unwrap();
        assert_eq!(serialized, "\"FD89D89D89D89D89D89D89D89D89D89D\"");
    }

    #[test]
    fn serde_deserialize_amount_hex() {
        let deserialized: AmountHex =
            serde_json::from_str("\"FD89D89D89D89D89D89D89D89D89D89D\"").unwrap();
        assert_eq!(
            deserialized,
            AmountHex::new(337010421085160209006996005437231978653)
        );
    }
}
