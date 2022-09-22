use std::{
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

use num_traits::FromPrimitive;

use crate::{
    deserialize_block_enum, serialize_block_enum,
    utils::{Deserialize, Serialize, Stream, StreamExt},
    Account, BlockEnum, BlockHash, SignatureVerification,
};

/// Information on an unchecked block
#[derive(Clone)]
pub struct UncheckedInfo {
    // todo: Remove Option as soon as no C++ code requires the empty constructor
    pub block: Option<Arc<RwLock<BlockEnum>>>,

    /// Seconds since posix epoch
    pub modified: u64,
    pub account: Account,
    pub verified: SignatureVerification,
}

impl UncheckedInfo {
    pub(crate) fn new(
        block: Arc<RwLock<BlockEnum>>,
        account: &Account,
        verified: SignatureVerification,
    ) -> Self {
        Self {
            block: Some(block),
            modified: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            account: *account,
            verified,
        }
    }

    pub(crate) fn null() -> Self {
        Self {
            block: None,
            modified: 0,
            account: *Account::zero(),
            verified: SignatureVerification::Unknown,
        }
    }
}

impl Serialize for UncheckedInfo {
    fn serialized_size() -> usize {
        0 //todo remove
    }

    fn serialize(&self, stream: &mut dyn Stream) -> anyhow::Result<()> {
        serialize_block_enum(stream, &self.block.as_ref().unwrap().read().unwrap())?;
        self.account.serialize(stream)?;
        stream.write_u64_ne(self.modified)?;
        stream.write_u8(self.verified as u8)
    }
}

impl Deserialize for UncheckedInfo {
    type Target = Self;

    fn deserialize(stream: &mut dyn Stream) -> anyhow::Result<Self::Target> {
        let block = deserialize_block_enum(stream)?;
        let account = Account::deserialize(stream)?;
        let modified = stream.read_u64_ne()?;
        let verified = SignatureVerification::from_u8(stream.read_u8()?)
            .ok_or_else(|| anyhow!("unvalid verification state"))?;
        Ok(Self {
            block: Some(Arc::new(RwLock::new(block))),
            modified,
            account,
            verified,
        })
    }
}

pub struct UncheckedKey {
    pub previous: BlockHash,
    pub hash: BlockHash,
}

impl UncheckedKey {
    pub fn to_bytes(&self) -> [u8; 64] {
        let mut result = [0; 64];
        result[..32].copy_from_slice(self.previous.as_bytes());
        result[32..].copy_from_slice(self.hash.as_bytes());
        result
    }
}

impl Deserialize for UncheckedKey {
    type Target = Self;

    fn deserialize(stream: &mut dyn Stream) -> anyhow::Result<Self::Target> {
        let previous = BlockHash::deserialize(stream)?;
        let hash = BlockHash::deserialize(stream)?;
        Ok(Self { previous, hash })
    }
}

impl Serialize for UncheckedKey {
    fn serialized_size() -> usize {
        BlockHash::serialized_size() * 2
    }

    fn serialize(&self, stream: &mut dyn Stream) -> anyhow::Result<()> {
        self.previous.serialize(stream)?;
        self.hash.serialize(stream)
    }
}
