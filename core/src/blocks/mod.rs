mod block_details;
pub use block_details::BlockDetails;

mod block_sideband;
pub use block_sideband::BlockSideband;

mod change_block;
use change_block::JsonChangeBlock;
pub use change_block::{valid_change_block_predecessor, ChangeBlock, ChangeHashables};

mod open_block;
use open_block::JsonOpenBlock;
pub use open_block::{OpenBlock, OpenHashables};

mod receive_block;
use receive_block::JsonReceiveBlock;
pub use receive_block::{valid_receive_block_predecessor, ReceiveBlock, ReceiveHashables};

mod send_block;
use send_block::JsonSendBlock;
pub use send_block::{valid_send_block_predecessor, SendBlock, SendHashables};

mod state_block;
use state_block::JsonStateBlock;
pub use state_block::{StateBlock, StateHashables};

mod builders;
pub use builders::*;

use crate::{
    utils::{BufferWriter, Deserialize, MemoryStream, Stream},
    Account, Amount, BlockHash, BlockHashBuilder, Epoch, Epochs, FullHash, Link, PrivateKey,
    PublicKey, QualifiedRoot, Root, Signature,
};
use num::FromPrimitive;
use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, LazyLock, RwLock},
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

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum BlockSubType {
    Send,
    Receive,
    Open,
    Change,
    Epoch,
}

impl BlockSubType {
    pub fn as_str(&self) -> &'static str {
        match self {
            BlockSubType::Send => "send",
            BlockSubType::Receive => "receive",
            BlockSubType::Open => "open",
            BlockSubType::Change => "change",
            BlockSubType::Epoch => "epoch",
        }
    }
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

pub trait BlockBase: FullHash {
    fn block_type(&self) -> BlockType;
    fn account_field(&self) -> Option<Account>;
    fn hash(&self) -> BlockHash;
    fn link_field(&self) -> Option<Link>;
    fn block_signature(&self) -> &Signature;
    fn set_block_signature(&mut self, signature: &Signature);
    fn work(&self) -> u64;
    fn set_work(&mut self, work: u64);
    fn previous(&self) -> BlockHash;
    fn serialize_without_block_type(&self, writer: &mut dyn BufferWriter);
    fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string(&self.json_representation())?)
    }
    fn json_representation(&self) -> JsonBlock;
    fn root(&self) -> Root;
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

impl<T: BlockBase> FullHash for T {
    fn full_hash(&self) -> BlockHash {
        BlockHashBuilder::new()
            .update(self.hash().as_bytes())
            .update(self.block_signature().as_bytes())
            .update(self.work().to_ne_bytes())
            .build()
    }
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
pub enum Block {
    LegacySend(SendBlock),
    LegacyReceive(ReceiveBlock),
    LegacyOpen(OpenBlock),
    LegacyChange(ChangeBlock),
    State(StateBlock),
}

impl Block {
    pub fn new_test_instance() -> Self {
        Self::State(StateBlock::new_test_instance())
    }

    pub fn new_test_open() -> Self {
        Self::State(StateBlock::new_test_open())
    }

    pub fn new_test_instance_with_key(key: impl Into<PrivateKey>) -> Self {
        Self::State(StateBlock::new_test_instance_with_key(key.into()))
    }

    pub fn block_type(&self) -> BlockType {
        self.as_block().block_type()
    }

    pub fn as_block_mut(&mut self) -> &mut dyn BlockBase {
        match self {
            Block::LegacySend(b) => b,
            Block::LegacyReceive(b) => b,
            Block::LegacyOpen(b) => b,
            Block::LegacyChange(b) => b,
            Block::State(b) => b,
        }
    }

    pub fn as_block(&self) -> &dyn BlockBase {
        match self {
            Block::LegacySend(b) => b,
            Block::LegacyReceive(b) => b,
            Block::LegacyOpen(b) => b,
            Block::LegacyChange(b) => b,
            Block::State(b) => b,
        }
    }

    pub fn is_open(&self) -> bool {
        match &self {
            Block::LegacyOpen(_) => true,
            Block::State(state) => state.previous().is_zero(),
            _ => false,
        }
    }

    pub fn is_legacy(&self) -> bool {
        !matches!(self, Block::State(_))
    }

    pub fn is_change(&self) -> bool {
        match self {
            Block::LegacyChange(_) => true,
            Block::State(state) => state.link().is_zero(),
            _ => false,
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

    pub fn serialize(&self, stream: &mut dyn BufferWriter) {
        let block_type = self.block_type() as u8;
        stream.write_u8_safe(block_type);
        self.serialize_without_block_type(stream);
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

    pub fn deserialize(stream: &mut dyn Stream) -> anyhow::Result<Block> {
        let block_type =
            BlockType::from_u8(stream.read_u8()?).ok_or_else(|| anyhow!("invalid block type"))?;
        Self::deserialize_block_type(block_type, stream)
    }
}

impl From<Block> for serde_json::Value {
    fn from(value: Block) -> Self {
        serde_json::to_value(value.json_representation()).unwrap()
    }
}

impl From<SavedBlock> for serde_json::Value {
    fn from(value: SavedBlock) -> Self {
        let mut result = serde_json::to_value(value.block.json_representation()).unwrap();
        if let serde_json::Value::Object(obj) = &mut result {
            obj.insert(
                "subtype".to_string(),
                serde_json::Value::String(value.subtype().as_str().to_owned()),
            );
        }
        result
    }
}

impl FullHash for Block {
    fn full_hash(&self) -> BlockHash {
        self.as_block().full_hash()
    }
}

impl Deref for Block {
    type Target = dyn BlockBase;

    fn deref(&self) -> &Self::Target {
        match self {
            Block::LegacySend(b) => b,
            Block::LegacyReceive(b) => b,
            Block::LegacyOpen(b) => b,
            Block::LegacyChange(b) => b,
            Block::State(b) => b,
        }
    }
}

impl DerefMut for Block {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Block::LegacySend(b) => b,
            Block::LegacyReceive(b) => b,
            Block::LegacyOpen(b) => b,
            Block::LegacyChange(b) => b,
            Block::State(b) => b,
        }
    }
}

impl serde::Serialize for Block {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let json = self.as_block().json_representation();
        json.serialize(serializer)
    }
}

#[derive(PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JsonBlock {
    Open(JsonOpenBlock),
    Change(JsonChangeBlock),
    Receive(JsonReceiveBlock),
    Send(JsonSendBlock),
    State(JsonStateBlock),
}

impl<'de> serde::Deserialize<'de> for Block {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let json_block = JsonBlock::deserialize(deserializer)?;
        Ok(json_block.into())
    }
}

impl From<JsonBlock> for Block {
    fn from(value: JsonBlock) -> Self {
        match value {
            JsonBlock::Open(open) => Block::LegacyOpen(open.into()),
            JsonBlock::Change(change) => Block::LegacyChange(change.into()),
            JsonBlock::Receive(receive) => Block::LegacyReceive(receive.into()),
            JsonBlock::Send(send) => Block::LegacySend(send.into()),
            JsonBlock::State(state) => Block::State(state.into()),
        }
    }
}

impl From<Block> for JsonBlock {
    fn from(value: Block) -> Self {
        value.as_block().json_representation()
    }
}

impl From<&Block> for JsonBlock {
    fn from(value: &Block) -> Self {
        value.as_block().json_representation()
    }
}

/// A block with additional data about that block (the "sideband")
/// which is only known when the block is saved.
/// The sideband contains additional data like block height, block time, etc.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavedBlock {
    block: Block,
    sideband: BlockSideband,
}

impl SavedBlock {
    pub fn new(block: Block, sideband: BlockSideband) -> Self {
        Self { block, sideband }
    }

    pub fn new_test_open_block() -> Self {
        let block = Block::new_test_open();
        let sideband = BlockSideband {
            height: 1,
            timestamp: 222222,
            successor: BlockHash::zero(),
            account: block.account_field().unwrap(),
            balance: block.balance_field().unwrap(),
            details: BlockDetails::new(Epoch::Epoch2, false, true, false),
            source_epoch: Epoch::Epoch0,
        };
        Self::new(block, sideband)
    }

    pub fn new_test_instance() -> Self {
        let block = Block::new_test_instance();
        let sideband = Self::test_sideband(&block);
        Self::new(block, sideband)
    }

    pub fn new_test_instance_with_key(key: impl Into<PrivateKey>) -> Self {
        let block = Block::new_test_instance_with_key(key);
        let sideband = Self::test_sideband(&block);
        Self::new(block, sideband)
    }

    fn test_sideband(block: &Block) -> BlockSideband {
        BlockSideband {
            height: 2,
            timestamp: 222222,
            successor: BlockHash::zero(),
            account: block.account_field().unwrap(),
            balance: block.balance_field().unwrap(),
            details: BlockDetails::new(Epoch::Epoch2, true, false, false),
            source_epoch: Epoch::Epoch0,
        }
    }

    pub fn set_sideband(&mut self, sideband: BlockSideband) {
        self.sideband = sideband;
    }

    pub fn account(&self) -> Account {
        match self.account_field() {
            Some(account) => account,
            None => self.sideband.account,
        }
    }

    pub fn height(&self) -> u64 {
        self.sideband.height
    }

    pub fn timestamp(&self) -> u64 {
        self.sideband.timestamp
    }

    pub fn subtype(&self) -> BlockSubType {
        self.sideband.details.subtype()
    }

    pub fn successor(&self) -> Option<BlockHash> {
        if self.sideband.successor.is_zero() {
            None
        } else {
            Some(self.sideband.successor)
        }
    }

    pub fn epoch(&self) -> Epoch {
        self.sideband.details.epoch
    }

    pub fn is_epoch(&self) -> bool {
        self.sideband.details.is_epoch
    }

    pub fn is_receive(&self) -> bool {
        self.sideband.details.is_receive
    }

    pub fn is_send(&self) -> bool {
        match &self.block {
            Block::LegacySend(_) => true,
            Block::State(_) => self.sideband.details.is_send,
            _ => false,
        }
    }

    pub fn source(&self) -> Option<BlockHash> {
        match &self.block {
            Block::LegacyOpen(i) => Some(i.source()),
            Block::LegacyReceive(i) => Some(i.source()),
            Block::State(i) => {
                if self.sideband.details.is_receive {
                    Some(i.link().into())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn source_epoch(&self) -> Epoch {
        self.sideband.source_epoch
    }

    pub fn destination(&self) -> Option<Account> {
        match &self.block {
            Block::LegacySend(i) => Some(*i.destination()),
            Block::State(i) => {
                if self.sideband.details.is_send {
                    Some(i.link().into())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn serialize_with_sideband(&self) -> Vec<u8> {
        let mut stream = MemoryStream::new();
        self.block.serialize(&mut stream);
        self.sideband
            .serialize(&mut stream, self.block.block_type());
        stream.to_vec()
    }

    pub fn balance(&self) -> Amount {
        match &self.block {
            Block::LegacySend(b) => b.balance(),
            Block::State(b) => b.balance(),
            _ => self.sideband.balance,
        }
    }

    pub fn details(&self) -> &BlockDetails {
        &self.sideband.details
    }

    pub fn sideband(&self) -> &BlockSideband {
        &self.sideband
    }

    /// There can be at most two dependencies per block, namely "previous" and "link/source".
    pub fn dependent_blocks(&self, epochs: &Epochs, genesis_account: &Account) -> DependentBlocks {
        match &self.block {
            Block::LegacySend(b) => b.dependent_blocks(),
            Block::LegacyChange(b) => b.dependent_blocks(),
            Block::LegacyReceive(b) => b.dependent_blocks(),
            Block::LegacyOpen(b) => b.dependent_blocks(genesis_account),
            Block::State(state) => {
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

impl Deref for SavedBlock {
    type Target = Block;

    fn deref(&self) -> &Self::Target {
        &self.block
    }
}

impl DerefMut for SavedBlock {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.block
    }
}

impl From<SavedBlock> for Block {
    fn from(value: SavedBlock) -> Self {
        value.block
    }
}

impl Deserialize for SavedBlock {
    type Target = Self;
    fn deserialize(stream: &mut dyn Stream) -> anyhow::Result<Self> {
        let mut block = Block::deserialize(stream)?;
        let mut sideband = BlockSideband::from_stream(stream, block.block_type())?;
        // BlockSideband does not serialize all data depending on the block type.
        // That's why we fill in the missing data here:
        match &block {
            Block::LegacySend(i) => {
                sideband.balance = i.balance();
                sideband.details = BlockDetails::new(Epoch::Epoch0, true, false, false)
            }
            Block::LegacyOpen(open) => {
                sideband.account = open.account();
                sideband.details = BlockDetails::new(Epoch::Epoch0, false, true, false)
            }
            Block::LegacyReceive(_) => {
                sideband.details = BlockDetails::new(Epoch::Epoch0, false, true, false)
            }
            Block::LegacyChange(_) => {
                sideband.details = BlockDetails::new(Epoch::Epoch0, false, false, false)
            }
            Block::State(state) => {
                sideband.account = state.account();
                sideband.balance = state.balance();
            }
        }
        Ok(SavedBlock { block, sideband })
    }
}

#[derive(Clone)]
pub enum SavedOrUnsavedBlock {
    Saved(SavedBlock),
    Unsaved(Block),
}

impl From<SavedOrUnsavedBlock> for Block {
    fn from(value: SavedOrUnsavedBlock) -> Self {
        match value {
            SavedOrUnsavedBlock::Saved(b) => b.into(),
            SavedOrUnsavedBlock::Unsaved(b) => b,
        }
    }
}

impl Deref for SavedOrUnsavedBlock {
    type Target = Block;

    fn deref(&self) -> &Self::Target {
        match self {
            SavedOrUnsavedBlock::Saved(b) => b,
            SavedOrUnsavedBlock::Unsaved(b) => b,
        }
    }
}

static DEV_PRIVATE_KEY_DATA: &str =
    "34F0A37AAD20F4A260F0A5B3CB3D7FB50673212263E58A380BC10474BB039CE4";
pub static DEV_PUBLIC_KEY_DATA: &str =
    "B0311EA55708D6A53C75CDBF88300259C6D018522FE3D4D0A242E431F9E8B6D0"; // xrb_3e3j5tkog48pnny9dmfzj1r16pg8t1e76dz5tmac6iq689wyjfpiij4txtdo
pub static DEV_GENESIS_KEY: LazyLock<PrivateKey> =
    LazyLock::new(|| PrivateKey::from_priv_key_hex(DEV_PRIVATE_KEY_DATA).unwrap());

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
        let block = BlockBuilder::legacy_open().build_saved();
        assert_serializable(block.into());
    }

    #[test]
    fn serialize_legacy_receive() {
        let block = BlockBuilder::legacy_receive().build();
        assert_serializable(block);
    }

    #[test]
    fn serialize_legacy_send() {
        let block = BlockBuilder::legacy_send().build();
        assert_serializable(block);
    }

    #[test]
    fn serialize_legacy_change() {
        let block = BlockBuilder::legacy_change().build();
        assert_serializable(block);
    }

    #[test]
    fn serialize_state() {
        let block = BlockBuilder::state().build();
        assert_serializable(block);
    }

    fn assert_serializable(block: Block) {
        let mut buffer = MemoryStream::new();
        block.serialize(&mut buffer);
        let deserialized = Block::deserialize(&mut buffer).unwrap();
        assert_eq!(deserialized, block);
    }
}
