mod block_details;
pub use block_details::BlockDetails;

mod block_sideband;
pub use block_sideband::BlockSideband;

mod change_block;
use change_block::JsonChangeBlock;
pub use change_block::{valid_change_block_predecessor, ChangeBlock, ChangeHashables};

mod open_block;
use once_cell::sync::Lazy;
use open_block::JsonOpenBlock;
pub use open_block::{OpenBlock, OpenHashables};

mod receive_block;
use receive_block::JsonReceiveBlock;
pub use receive_block::{valid_receive_block_predecessor, ReceiveBlock, ReceiveHashables};

mod send_block;
use send_block::JsonSendBlock;
pub use send_block::{valid_send_block_predecessor, SendBlock, SendHashables};

mod state_block;
use serde::{Deserialize, Serialize};
use state_block::JsonStateBlock;
pub use state_block::{StateBlock, StateHashables};

mod builders;
pub use builders::*;

use crate::{
    utils::{BufferReader, BufferWriter, MemoryStream, PropertyTree, SerdePropertyTree, Stream},
    Account, Amount, BlockHash, BlockHashBuilder, Epoch, Epochs, FullHash, KeyPair, Link,
    PublicKey, QualifiedRoot, Root, Signature, WorkVersion,
};
use num::FromPrimitive;
use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, RwLock},
};

#[repr(u8)]
#[derive(PartialEq, Eq, Debug, Clone, Copy, FromPrimitive)]
pub enum BlockType {
    Invalid = 0,
    NotABlock = 1,
    LegacySend = 2,
    LegacyReceive = 3,
    LegacyOpen = 4,
    LegacyChange = 5,
    State = 6,
}

impl TryFrom<BlockType> for BlockSubType {
    type Error = anyhow::Error;

    fn try_from(value: BlockType) -> Result<Self, Self::Error> {
        match value {
            BlockType::LegacySend => Ok(BlockSubType::Send),
            BlockType::LegacyReceive => Ok(BlockSubType::Receive),
            BlockType::LegacyOpen => Ok(BlockSubType::Open),
            BlockType::LegacyChange => Ok(BlockSubType::Change),
            BlockType::State => Ok(BlockSubType::Send),
            BlockType::Invalid | BlockType::NotABlock => {
                Err(anyhow!("Invalid block type for conversion to subtype"))
            }
        }
    }
}

impl TryFrom<u8> for BlockType {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        FromPrimitive::from_u8(value).ok_or_else(|| anyhow!("invalid block type value"))
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BlockSubType {
    Send,
    Receive,
    Open,
    Change,
    Epoch,
}

#[derive(Clone, Default)]
pub struct LazyBlockHash {
    // todo: Remove Arc<RwLock>? Maybe remove lazy hash calculation?
    hash: Arc<RwLock<BlockHash>>,
}

impl LazyBlockHash {
    pub fn new() -> Self {
        Self {
            hash: Arc::new(RwLock::new(BlockHash::zero())),
        }
    }
    pub fn hash(&'_ self, factory: impl Into<BlockHash>) -> BlockHash {
        let mut value = self.hash.read().unwrap();
        if value.is_zero() {
            drop(value);
            let mut x = self.hash.write().unwrap();
            let block_hash: BlockHash = factory.into();
            *x = block_hash;
            drop(x);
            value = self.hash.read().unwrap();
        }

        *value
    }

    pub fn clear(&self) {
        let mut x = self.hash.write().unwrap();
        *x = BlockHash::zero();
    }
}

impl std::fmt::Debug for LazyBlockHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self.hash.read().unwrap().deref(), f)
    }
}

pub trait Block: FullHash {
    fn block_type(&self) -> BlockType;
    fn account_field(&self) -> Option<Account>;

    /**
     * Contextual details about a block, some fields may or may not be set depending on block type.
     * This field is set via sideband_set in ledger processing or deserializing blocks from the database.
     * Otherwise it may be null (for example, an old block or fork).
     */
    fn sideband(&'_ self) -> Option<&'_ BlockSideband>;
    fn set_sideband(&mut self, sideband: BlockSideband);
    fn hash(&self) -> BlockHash;
    fn link_field(&self) -> Option<Link>;
    fn block_signature(&self) -> &Signature;
    fn set_block_signature(&mut self, signature: &Signature);
    fn work(&self) -> u64;
    fn set_work(&mut self, work: u64);
    fn previous(&self) -> BlockHash;
    fn serialize_without_block_type(&self, writer: &mut dyn BufferWriter);
    fn serialize_json(&self, writer: &mut dyn PropertyTree) -> anyhow::Result<()>;
    fn to_json(&self) -> anyhow::Result<String> {
        let mut writer = SerdePropertyTree::new();
        self.serialize_json(&mut writer)?;
        Ok(writer.to_json())
    }
    fn json_representation(&self) -> JsonBlock;
    fn work_version(&self) -> WorkVersion {
        WorkVersion::Work1
    }
    fn root(&self) -> Root;
    fn visit(&self, visitor: &mut dyn BlockVisitor);
    fn visit_mut(&mut self, visitor: &mut dyn MutableBlockVisitor);
    fn balance_field(&self) -> Option<Amount>;
    /// Source block for open/receive blocks, zero otherwise.
    fn source_field(&self) -> Option<BlockHash>;
    fn representative_field(&self) -> Option<PublicKey>;
    fn destination_field(&self) -> Option<Account>;
    fn qualified_root(&self) -> QualifiedRoot {
        QualifiedRoot::new(self.root(), self.previous())
    }
    fn valid_predecessor(&self, block_type: BlockType) -> bool;
}

impl<T: Block> FullHash for T {
    fn full_hash(&self) -> BlockHash {
        BlockHashBuilder::new()
            .update(self.hash().as_bytes())
            .update(self.block_signature().as_bytes())
            .update(self.work().to_ne_bytes())
            .build()
    }
}

pub trait BlockVisitor {
    fn send_block(&mut self, block: &SendBlock);
    fn receive_block(&mut self, block: &ReceiveBlock);
    fn open_block(&mut self, block: &OpenBlock);
    fn change_block(&mut self, block: &ChangeBlock);
    fn state_block(&mut self, block: &StateBlock);
}

pub trait MutableBlockVisitor {
    fn send_block(&mut self, block: &mut SendBlock);
    fn receive_block(&mut self, block: &mut ReceiveBlock);
    fn open_block(&mut self, block: &mut OpenBlock);
    fn change_block(&mut self, block: &mut ChangeBlock);
    fn state_block(&mut self, block: &mut StateBlock);
}

pub fn serialized_block_size(block_type: BlockType) -> usize {
    match block_type {
        BlockType::Invalid | BlockType::NotABlock => 0,
        BlockType::LegacySend => SendBlock::serialized_size(),
        BlockType::LegacyReceive => ReceiveBlock::serialized_size(),
        BlockType::LegacyOpen => OpenBlock::serialized_size(),
        BlockType::LegacyChange => ChangeBlock::serialized_size(),
        BlockType::State => StateBlock::serialized_size(),
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum BlockEnum {
    LegacySend(SendBlock),
    LegacyReceive(ReceiveBlock),
    LegacyOpen(OpenBlock),
    LegacyChange(ChangeBlock),
    State(StateBlock),
}

impl BlockEnum {
    pub fn new_test_instance() -> Self {
        Self::State(StateBlock::new_test_instance())
    }

    pub fn new_test_instance_with_key(key: impl Into<KeyPair>) -> Self {
        Self::State(StateBlock::new_test_instance_with_key(key.into()))
    }

    pub fn block_type(&self) -> BlockType {
        self.as_block().block_type()
    }

    pub fn as_block_mut(&mut self) -> &mut dyn Block {
        match self {
            BlockEnum::LegacySend(b) => b,
            BlockEnum::LegacyReceive(b) => b,
            BlockEnum::LegacyOpen(b) => b,
            BlockEnum::LegacyChange(b) => b,
            BlockEnum::State(b) => b,
        }
    }

    pub fn as_block(&self) -> &dyn Block {
        match self {
            BlockEnum::LegacySend(b) => b,
            BlockEnum::LegacyReceive(b) => b,
            BlockEnum::LegacyOpen(b) => b,
            BlockEnum::LegacyChange(b) => b,
            BlockEnum::State(b) => b,
        }
    }

    pub fn balance(&self) -> Amount {
        match self {
            BlockEnum::LegacySend(b) => b.balance(),
            BlockEnum::LegacyReceive(b) => b.sideband().unwrap().balance,
            BlockEnum::LegacyOpen(b) => b.sideband().unwrap().balance,
            BlockEnum::LegacyChange(b) => b.sideband().unwrap().balance,
            BlockEnum::State(b) => b.balance(),
        }
    }

    pub fn is_open(&self) -> bool {
        match &self {
            BlockEnum::LegacyOpen(_) => true,
            BlockEnum::State(state) => state.previous().is_zero(),
            _ => false,
        }
    }

    pub fn is_legacy(&self) -> bool {
        !matches!(self, BlockEnum::State(_))
    }

    pub fn is_epoch(&self) -> bool {
        match self {
            BlockEnum::State(_) => self.sideband().unwrap().details.is_epoch,
            _ => false,
        }
    }

    pub fn is_send(&self) -> bool {
        match self {
            BlockEnum::LegacySend(_) => true,
            BlockEnum::State(_) => self.sideband().unwrap().details.is_send,
            _ => false,
        }
    }

    pub fn is_receive(&self) -> bool {
        match self {
            BlockEnum::LegacyReceive(_) | BlockEnum::LegacyOpen(_) => true,
            BlockEnum::State(_) => self.sideband().unwrap().details.is_receive,
            _ => false,
        }
    }

    pub fn is_change(&self) -> bool {
        match self {
            BlockEnum::LegacyChange(_) => true,
            BlockEnum::State(state) => state.link().is_zero(),
            _ => false,
        }
    }

    pub fn source(&self) -> Option<BlockHash> {
        match self {
            BlockEnum::LegacyOpen(i) => Some(i.source()),
            BlockEnum::LegacyReceive(i) => Some(i.source()),
            BlockEnum::State(i) => {
                if i.sideband().unwrap().details.is_receive {
                    Some(i.link().into())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn destination(&self) -> Option<Account> {
        match self {
            BlockEnum::LegacySend(i) => Some(*i.destination()),
            BlockEnum::State(i) => {
                if i.sideband().unwrap().details.is_send {
                    Some(i.link().into())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn source_or_link(&self) -> BlockHash {
        self.source_field()
            .unwrap_or_else(|| self.link_field().unwrap_or_default().into())
    }

    pub fn destination_or_link(&self) -> Account {
        self.destination_field()
            .unwrap_or_else(|| self.link_field().unwrap_or_default().into())
    }

    pub fn account(&self) -> Account {
        match self.account_field() {
            Some(account) => account,
            None => self.sideband().unwrap().account,
        }
    }

    pub fn height(&self) -> u64 {
        self.sideband().map(|s| s.height).unwrap_or_default()
    }

    pub fn successor(&self) -> Option<BlockHash> {
        if let Some(sideband) = self.sideband() {
            if !sideband.successor.is_zero() {
                Some(sideband.successor)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn epoch(&self) -> Epoch {
        self.sideband().unwrap().details.epoch
    }

    pub fn serialize(&self, stream: &mut dyn BufferWriter) {
        let block_type = self.block_type() as u8;
        stream.write_u8_safe(block_type);
        self.serialize_without_block_type(stream);
    }

    pub fn serialize_with_sideband(&self) -> Vec<u8> {
        let mut stream = MemoryStream::new();
        self.serialize(&mut stream);
        self.sideband()
            .unwrap()
            .serialize(&mut stream, self.block_type());
        stream.to_vec()
    }

    pub fn deserialize_with_sideband(bytes: &[u8]) -> anyhow::Result<BlockEnum> {
        let mut stream = BufferReader::new(bytes);
        let mut block = BlockEnum::deserialize(&mut stream)?;
        let mut sideband = BlockSideband::from_stream(&mut stream, block.block_type())?;
        // BlockSideband does not serialize all data depending on the block type.
        // That's why we fill in the missing data here:
        match &block {
            BlockEnum::LegacySend(i) => {
                sideband.balance = i.balance();
                sideband.details = BlockDetails::new(Epoch::Epoch0, true, false, false)
            }
            BlockEnum::LegacyOpen(open) => {
                sideband.account = open.account();
                sideband.details = BlockDetails::new(Epoch::Epoch0, false, true, false)
            }
            BlockEnum::LegacyReceive(_) => {
                sideband.details = BlockDetails::new(Epoch::Epoch0, false, true, false)
            }
            BlockEnum::LegacyChange(_) => {
                sideband.details = BlockDetails::new(Epoch::Epoch0, false, false, false)
            }
            BlockEnum::State(state) => {
                sideband.account = state.account();
                sideband.balance = state.balance();
            }
        }
        block.as_block_mut().set_sideband(sideband);
        Ok(block)
    }

    pub fn deserialize_block_type(
        block_type: BlockType,
        stream: &mut dyn Stream,
    ) -> anyhow::Result<Self> {
        let block = match block_type {
            BlockType::LegacyReceive => Self::LegacyReceive(ReceiveBlock::deserialize(stream)?),
            BlockType::LegacyOpen => Self::LegacyOpen(OpenBlock::deserialize(stream)?),
            BlockType::LegacyChange => Self::LegacyChange(ChangeBlock::deserialize(stream)?),
            BlockType::State => Self::State(StateBlock::deserialize(stream)?),
            BlockType::LegacySend => Self::LegacySend(SendBlock::deserialize(stream)?),
            BlockType::Invalid | BlockType::NotABlock => bail!("invalid block type"),
        };
        Ok(block)
    }

    pub fn deserialize(stream: &mut dyn Stream) -> anyhow::Result<BlockEnum> {
        let block_type =
            BlockType::from_u8(stream.read_u8()?).ok_or_else(|| anyhow!("invalid block type"))?;
        Self::deserialize_block_type(block_type, stream)
    }

    /// There can be at most two dependencies per block, namely "previous" and "link/source".
    pub fn dependent_blocks(&self, epochs: &Epochs, genensis_account: &Account) -> DependentBlocks {
        match self {
            BlockEnum::LegacySend(_) | BlockEnum::LegacyChange(_) => {
                DependentBlocks::new(self.previous(), BlockHash::zero())
            }
            BlockEnum::LegacyReceive(receive) => {
                DependentBlocks::new(receive.previous(), receive.source())
            }
            BlockEnum::LegacyOpen(open) => {
                if &open.account() == genensis_account {
                    DependentBlocks::none()
                } else {
                    DependentBlocks::new(open.source(), BlockHash::zero())
                }
            }
            BlockEnum::State(state) => {
                let link_refers_to_block = !self.is_send() && !epochs.is_epoch_link(&state.link());
                let linked_block = if link_refers_to_block {
                    state.link().into()
                } else {
                    BlockHash::zero()
                };
                DependentBlocks::new(self.previous(), linked_block)
            }
        }
    }
}

impl FullHash for BlockEnum {
    fn full_hash(&self) -> BlockHash {
        self.as_block().full_hash()
    }
}

impl Deref for BlockEnum {
    type Target = dyn Block;

    fn deref(&self) -> &Self::Target {
        match self {
            BlockEnum::LegacySend(b) => b,
            BlockEnum::LegacyReceive(b) => b,
            BlockEnum::LegacyOpen(b) => b,
            BlockEnum::LegacyChange(b) => b,
            BlockEnum::State(b) => b,
        }
    }
}

impl DerefMut for BlockEnum {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            BlockEnum::LegacySend(b) => b,
            BlockEnum::LegacyReceive(b) => b,
            BlockEnum::LegacyOpen(b) => b,
            BlockEnum::LegacyChange(b) => b,
            BlockEnum::State(b) => b,
        }
    }
}

impl serde::Serialize for BlockEnum {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let json = self.as_block().json_representation();
        json.serialize(serializer)
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JsonBlock {
    Open(JsonOpenBlock),
    Change(JsonChangeBlock),
    Receive(JsonReceiveBlock),
    Send(JsonSendBlock),
    State(JsonStateBlock),
}

impl<'de> serde::Deserialize<'de> for BlockEnum {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let json_block = JsonBlock::deserialize(deserializer)?;
        Ok(json_block.into())
    }
}

impl From<JsonBlock> for BlockEnum {
    fn from(value: JsonBlock) -> Self {
        match value {
            JsonBlock::Open(open) => BlockEnum::LegacyOpen(open.into()),
            JsonBlock::Change(change) => BlockEnum::LegacyChange(change.into()),
            JsonBlock::Receive(receive) => BlockEnum::LegacyReceive(receive.into()),
            JsonBlock::Send(send) => BlockEnum::LegacySend(send.into()),
            JsonBlock::State(state) => BlockEnum::State(state.into()),
        }
    }
}

impl From<BlockEnum> for JsonBlock {
    fn from(value: BlockEnum) -> Self {
        value.as_block().json_representation()
    }
}

impl From<&BlockEnum> for JsonBlock {
    fn from(value: &BlockEnum) -> Self {
        value.as_block().json_representation()
    }
}

pub fn deserialize_block_json(ptree: &impl PropertyTree) -> anyhow::Result<BlockEnum> {
    let block_type = ptree.get_string("type")?;
    match block_type.as_str() {
        "receive" => ReceiveBlock::deserialize_json(ptree).map(BlockEnum::LegacyReceive),
        "send" => SendBlock::deserialize_json(ptree).map(BlockEnum::LegacySend),
        "open" => OpenBlock::deserialize_json(ptree).map(BlockEnum::LegacyOpen),
        "change" => ChangeBlock::deserialize_json(ptree).map(BlockEnum::LegacyChange),
        "state" => StateBlock::deserialize_json(ptree).map(BlockEnum::State),
        _ => Err(anyhow!("unsupported block type")),
    }
}

pub struct BlockWithSideband {
    pub block: BlockEnum,
    pub sideband: BlockSideband,
}

impl crate::utils::Deserialize for BlockWithSideband {
    type Target = Self;
    fn deserialize(stream: &mut dyn Stream) -> anyhow::Result<Self> {
        let mut block = BlockEnum::deserialize(stream)?;
        let sideband = BlockSideband::from_stream(stream, block.block_type())?;
        block.as_block_mut().set_sideband(sideband.clone());
        Ok(BlockWithSideband { block, sideband })
    }
}

pub trait HackyUnsafeMutBlock {
    unsafe fn undefined_behavior_mut(&self) -> &mut BlockEnum;
}

impl HackyUnsafeMutBlock for Arc<BlockEnum> {
    unsafe fn undefined_behavior_mut(&self) -> &mut BlockEnum {
        // This is undefined behavior and has to be changed to a proper implementation ASAP
        let block_ptr = Arc::as_ptr(self) as *mut BlockEnum;
        &mut *block_ptr
    }
}

static DEV_PRIVATE_KEY_DATA: &str =
    "34F0A37AAD20F4A260F0A5B3CB3D7FB50673212263E58A380BC10474BB039CE4";
pub static DEV_PUBLIC_KEY_DATA: &str =
    "B0311EA55708D6A53C75CDBF88300259C6D018522FE3D4D0A242E431F9E8B6D0"; // xrb_3e3j5tkog48pnny9dmfzj1r16pg8t1e76dz5tmac6iq689wyjfpiij4txtdo
pub static DEV_GENESIS_KEY: Lazy<KeyPair> =
    Lazy::new(|| KeyPair::from_priv_key_hex(DEV_PRIVATE_KEY_DATA).unwrap());

#[derive(Default)]
pub struct DependentBlocks {
    dependents: [BlockHash; 2],
}

impl DependentBlocks {
    pub fn new(previous: BlockHash, link: BlockHash) -> Self {
        Self {
            dependents: [previous, link],
        }
    }

    pub fn none() -> Self {
        Self::new(BlockHash::zero(), BlockHash::zero())
    }

    pub fn previous(&self) -> Option<BlockHash> {
        self.get_index(0)
    }

    pub fn link(&self) -> Option<BlockHash> {
        self.get_index(1)
    }

    fn get_index(&self, index: usize) -> Option<BlockHash> {
        if self.dependents[index].is_zero() {
            None
        } else {
            Some(self.dependents[index])
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &BlockHash> {
        self.dependents
            .iter()
            .flat_map(|i| if i.is_zero() { None } else { Some(i) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_legacy_open() {
        let block = BlockBuilder::legacy_open().with_sideband().build();
        assert_serializable(block);
    }

    #[test]
    fn serialize_legacy_receive() {
        let block = BlockBuilder::legacy_receive().with_sideband().build();
        assert_serializable(block);
    }

    #[test]
    fn serialize_legacy_send() {
        let block = BlockBuilder::legacy_send().with_sideband().build();
        assert_serializable(block);
    }

    #[test]
    fn serialize_legacy_change() {
        let block = BlockBuilder::legacy_change().with_sideband().build();
        assert_serializable(block);
    }

    #[test]
    fn serialize_state() {
        let block = BlockBuilder::state().with_sideband().build();
        assert_serializable(block);
    }

    fn assert_serializable(block: BlockEnum) {
        let bytes = block.serialize_with_sideband();
        let deserialized = BlockEnum::deserialize_with_sideband(&bytes).unwrap();

        assert_eq!(deserialized, block);
    }
}
