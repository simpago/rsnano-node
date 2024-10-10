#![allow(clippy::missing_safety_doc)]

#[macro_use]
extern crate anyhow;

#[macro_use]
extern crate num_derive;

#[macro_use]
extern crate static_assertions;

mod account;
mod amount;
mod block_hash;
mod vote;

pub use account::Account;
pub use amount::{Amount, GXRB_RATIO, KXRB_RATIO, MXRB_RATIO, XRB_RATIO};
use blake2::{
    digest::{Update, VariableOutput},
    Blake2bVar,
};
pub use block_hash::{BlockHash, BlockHashBuilder};
use rand::{thread_rng, Rng};
use serde::de::{Unexpected, Visitor};
pub use vote::*;

mod key_pair;
pub use key_pair::{
    sign_message, validate_block_signature, validate_message, KeyPair, KeyPairFactory,
};

mod raw_key;
pub use raw_key::RawKey;

mod signature;
pub use signature::Signature;

mod u256_struct;

pub mod utils;

mod qualified_root;
pub use qualified_root::QualifiedRoot;

mod account_info;
pub use account_info::AccountInfo;

mod epoch;
pub use epoch::{Epoch, Epochs};

mod confirmation_height_info;
pub use confirmation_height_info::ConfirmationHeightInfo;

mod pending_key;
pub use pending_key::PendingKey;

mod pending_info;
pub use pending_info::PendingInfo;

mod difficulty;
pub use difficulty::{Difficulty, DifficultyV1, StubDifficulty, WorkVersion};

mod blocks;
pub use blocks::*;

pub mod work;

mod unchecked_info;
pub use unchecked_info::{UncheckedInfo, UncheckedKey};

mod kdf;
pub use kdf::KeyDerivationFunction;
use utils::{BufferWriter, Deserialize, Serialize, Stream};

use std::{
    fmt::{Debug, Display, Write},
    str::FromStr,
    sync::Mutex,
};
use std::{num::ParseIntError, sync::LazyLock};

pub fn encode_hex(i: u128) -> String {
    let mut result = String::with_capacity(32);
    for byte in i.to_be_bytes() {
        write!(&mut result, "{:02X}", byte).unwrap();
    }
    result
}

pub fn write_hex_bytes(bytes: &[u8], f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
    for &byte in bytes {
        write!(f, "{:02X}", byte)?;
    }
    Ok(())
}

pub fn to_hex_string(i: u64) -> String {
    format!("{:016X}", i)
}

pub fn u64_from_hex_str(s: impl AsRef<str>) -> Result<u64, ParseIntError> {
    u64::from_str_radix(s.as_ref(), 16)
}

u256_struct!(HashOrAccount);
serialize_32_byte_string!(HashOrAccount);
u256_struct!(Link);
serialize_32_byte_string!(Link);
u256_struct!(PublicKey);
serialize_32_byte_string!(PublicKey);
u256_struct!(Root);
serialize_32_byte_string!(Root);
u256_struct!(WalletId);
serialize_32_byte_string!(WalletId);

impl WalletId {
    pub fn random() -> Self {
        let secret: [u8; 32] = thread_rng().gen();
        let keys = KeyPair::from_priv_key_bytes(&secret).unwrap();
        Self::from_bytes(*keys.public_key().as_bytes())
    }
}

impl From<HashOrAccount> for Account {
    fn from(source: HashOrAccount) -> Self {
        Account::from_bytes(*source.as_bytes())
    }
}

impl From<&HashOrAccount> for Account {
    fn from(source: &HashOrAccount) -> Self {
        Account::from_bytes(*source.as_bytes())
    }
}

impl From<Link> for Account {
    fn from(link: Link) -> Self {
        Account::from_bytes(*link.as_bytes())
    }
}

impl From<&Link> for Account {
    fn from(link: &Link) -> Self {
        Account::from_bytes(*link.as_bytes())
    }
}

impl From<Root> for Account {
    fn from(root: Root) -> Self {
        Account::from_bytes(*root.as_bytes())
    }
}

impl From<Account> for Link {
    fn from(account: Account) -> Self {
        Link::from_bytes(*account.as_bytes())
    }
}

impl From<&Account> for Link {
    fn from(account: &Account) -> Self {
        Link::from_bytes(*account.as_bytes())
    }
}

impl From<BlockHash> for Link {
    fn from(hash: BlockHash) -> Self {
        Link::from_bytes(*hash.as_bytes())
    }
}

impl From<HashOrAccount> for BlockHash {
    fn from(source: HashOrAccount) -> Self {
        BlockHash::from_bytes(*source.as_bytes())
    }
}

impl From<&HashOrAccount> for BlockHash {
    fn from(source: &HashOrAccount) -> Self {
        BlockHash::from_bytes(*source.as_bytes())
    }
}
impl From<Link> for BlockHash {
    fn from(link: Link) -> Self {
        BlockHash::from_bytes(*link.as_bytes())
    }
}

impl From<Root> for BlockHash {
    fn from(root: Root) -> Self {
        BlockHash::from_bytes(*root.as_bytes())
    }
}

impl From<Account> for HashOrAccount {
    fn from(account: Account) -> Self {
        HashOrAccount::from_bytes(*account.as_bytes())
    }
}

impl From<&BlockHash> for HashOrAccount {
    fn from(hash: &BlockHash) -> Self {
        HashOrAccount::from_bytes(*hash.as_bytes())
    }
}

impl From<BlockHash> for HashOrAccount {
    fn from(hash: BlockHash) -> Self {
        HashOrAccount::from_bytes(*hash.as_bytes())
    }
}

impl From<Link> for HashOrAccount {
    fn from(link: Link) -> Self {
        HashOrAccount::from_bytes(*link.as_bytes())
    }
}

impl From<&Link> for HashOrAccount {
    fn from(link: &Link) -> Self {
        HashOrAccount::from_bytes(*link.as_bytes())
    }
}

impl From<Account> for Root {
    fn from(key: Account) -> Self {
        Root::from_bytes(*key.as_bytes())
    }
}

impl From<&PublicKey> for Root {
    fn from(key: &PublicKey) -> Self {
        Root::from_bytes(*key.as_bytes())
    }
}

impl From<PublicKey> for Root {
    fn from(key: PublicKey) -> Self {
        Root::from_bytes(*key.as_bytes())
    }
}

impl From<&Account> for Root {
    fn from(hash: &Account) -> Self {
        Root::from_bytes(*hash.as_bytes())
    }
}

impl From<BlockHash> for Root {
    fn from(hash: BlockHash) -> Self {
        Root::from_bytes(*hash.as_bytes())
    }
}

impl From<&BlockHash> for Root {
    fn from(hash: &BlockHash) -> Self {
        Root::from_bytes(*hash.as_bytes())
    }
}

impl PublicKey {
    /// IV for Key encryption
    pub fn initialization_vector(&self) -> [u8; 16] {
        self.0[..16].try_into().unwrap()
    }

    pub fn to_node_id(&self) -> String {
        Account::from(self).to_node_id()
    }

    pub fn as_account(&self) -> Account {
        self.into()
    }
}

impl TryFrom<&RawKey> for PublicKey {
    type Error = anyhow::Error;
    fn try_from(prv: &RawKey) -> Result<Self, Self::Error> {
        let secret = ed25519_dalek::SecretKey::from(*prv.as_bytes());
        let signing_key = ed25519_dalek::SigningKey::from(&secret);
        let public = ed25519_dalek::VerifyingKey::from(&signing_key);
        Ok(PublicKey::from_bytes(public.to_bytes()))
    }
}

pub trait FullHash {
    fn full_hash(&self) -> BlockHash;
}

#[derive(PartialEq, Eq, Debug, Copy, Clone, PartialOrd, Ord)]
pub struct NoValue {}

impl utils::FixedSizeSerialize for NoValue {
    fn serialized_size() -> usize {
        0
    }
}

impl utils::Serialize for NoValue {
    fn serialize(&self, _writer: &mut dyn BufferWriter) {}
}

impl utils::Deserialize for NoValue {
    type Target = Self;
    fn deserialize(_stream: &mut dyn utils::Stream) -> anyhow::Result<NoValue> {
        Ok(NoValue {})
    }
}

pub fn deterministic_key(seed: &RawKey, index: u32) -> RawKey {
    let mut buffer = [0; 32];
    let mut hasher = Blake2bVar::new(buffer.len()).unwrap();
    hasher.update(seed.as_bytes());
    hasher.update(&index.to_be_bytes());
    hasher.finalize_variable(&mut buffer).unwrap();
    RawKey::from_bytes(buffer)
}

/**
 * Network variants with different genesis blocks and network parameters
 */
#[repr(u16)]
#[derive(Clone, Copy, FromPrimitive, PartialEq, Eq, Debug)]
pub enum Networks {
    Invalid = 0x0,
    // Low work parameters, publicly known genesis key, dev IP ports
    NanoDevNetwork = 0x5241, // 'R', 'A'
    // Normal work parameters, secret beta genesis key, beta IP ports
    NanoBetaNetwork = 0x5242, // 'R', 'B'
    // Normal work parameters, secret live key, live IP ports
    NanoLiveNetwork = 0x5243, // 'R', 'C'
    // Normal work parameters, secret test genesis key, test IP ports
    NanoTestNetwork = 0x5258, // 'R', 'X'
}

impl Networks {
    pub fn as_str(&self) -> &str {
        match self {
            Networks::Invalid => "invalid",
            Networks::NanoDevNetwork => "dev",
            Networks::NanoBetaNetwork => "beta",
            Networks::NanoLiveNetwork => "live",
            Networks::NanoTestNetwork => "test",
        }
    }
}

impl FromStr for Networks {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Networks, Self::Err> {
        match s {
            "dev" => Ok(Networks::NanoDevNetwork),
            "beta" => Ok(Networks::NanoBetaNetwork),
            "live" => Ok(Networks::NanoLiveNetwork),
            "test" => Ok(Networks::NanoTestNetwork),
            _ => Err("Invalid network"),
        }
    }
}
//
//todo: make configurable in builld script again!
pub static ACTIVE_NETWORK: LazyLock<Mutex<Networks>> =
    LazyLock::new(|| Mutex::new(Networks::NanoBetaNetwork));

pub fn epoch_v1_link() -> Link {
    let mut link_bytes = [0u8; 32];
    link_bytes[..14].copy_from_slice(b"epoch v1 block");
    Link::from_bytes(link_bytes)
}

pub fn epoch_v2_link() -> Link {
    let mut link_bytes = [0u8; 32];
    link_bytes[..14].copy_from_slice(b"epoch v2 block");
    Link::from_bytes(link_bytes)
}

#[derive(PartialEq, Eq, Debug, Default, Clone)]
pub struct Frontier {
    pub account: Account,
    pub hash: BlockHash,
}

impl Frontier {
    pub fn new(account: Account, hash: BlockHash) -> Self {
        Self { account, hash }
    }
}

impl Frontier {
    pub fn deserialize(stream: &mut dyn Stream) -> anyhow::Result<Self> {
        let account = Account::deserialize(stream)?;
        let hash = BlockHash::deserialize(stream)?;
        Ok(Self::new(account, hash))
    }
}

impl Serialize for Frontier {
    fn serialize(&self, stream: &mut dyn BufferWriter) {
        self.account.serialize(stream);
        self.hash.serialize(stream);
    }
}

#[derive(PartialEq, Eq, Copy, Clone, PartialOrd, Ord)]
pub struct WorkNonce(u64);

impl Display for WorkNonce {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:016X}", self.0)
    }
}

impl Debug for WorkNonce {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self, f)
    }
}

impl From<u64> for WorkNonce {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<WorkNonce> for u64 {
    fn from(value: WorkNonce) -> Self {
        value.0
    }
}

impl serde::Serialize for WorkNonce {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&to_hex_string(self.0))
    }
}

impl<'de> serde::Deserialize<'de> for WorkNonce {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = deserializer.deserialize_str(WorkNonceVisitor {})?;
        Ok(value)
    }
}

struct WorkNonceVisitor {}

impl<'de> Visitor<'de> for WorkNonceVisitor {
    type Value = WorkNonce;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a hex string containing 8 bytes")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let mut bytes = [0; 8];
        hex::decode_to_slice(v, &mut bytes).map_err(|_| {
            serde::de::Error::invalid_value(Unexpected::Str(v), &"a hex string containing 8 bytes")
        })?;
        Ok(WorkNonce(u64::from_be_bytes(bytes)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_key() {
        let seed = RawKey::from(1);
        let key = deterministic_key(&seed, 3);
        assert_eq!(
            key,
            RawKey::decode_hex("89A518E3B70A0843DE8470F87FF851F9C980B1B2802267A05A089677B8FA1926")
                .unwrap()
        );
    }
}
