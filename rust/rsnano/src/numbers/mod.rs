mod account;
mod difficulty;

use std::convert::TryInto;
use std::fmt::Write;
use std::ops::Deref;
use std::{convert::TryFrom, fmt::Display};

use crate::utils::Stream;
use anyhow::Result;

pub use account::*;
use blake2::digest::{Update, VariableOutput};
pub use difficulty::*;
use once_cell::sync::Lazy;

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug, Hash)]
pub struct PublicKey {
    value: [u8; 32], // big endian
}

impl PublicKey {
    pub fn new() -> Self {
        Self { value: [0; 32] }
    }

    pub fn is_zero(&self) -> bool {
        self.value == [0; 32]
    }

    pub fn from_bytes(value: [u8; 32]) -> Self {
        Self { value }
    }

    pub fn try_from_bytes(value: &[u8]) -> Result<Self> {
        Ok(Self::from_bytes(value.try_into()?))
    }

    pub const fn serialized_size() -> usize {
        32
    }

    pub fn serialize(&self, stream: &mut dyn Stream) -> Result<()> {
        stream.write_bytes(&self.value)
    }

    pub fn deserialize(stream: &mut impl Stream) -> Result<Self> {
        let mut result = PublicKey::new();
        stream.read_bytes(&mut result.value, 32)?;
        Ok(result)
    }

    pub fn as_bytes(&'_ self) -> &'_ [u8; 32] {
        &self.value
    }

    pub fn to_be_bytes(self) -> [u8; 32] {
        self.value
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug, Hash)]
pub struct BlockHash {
    value: [u8; 32], //big endian
}

const ZERO_BLOCK_HASH: BlockHash = BlockHash { value: [0; 32] };

impl BlockHash {
    pub fn new() -> Self {
        Self { value: [0; 32] }
    }

    pub fn zero() -> &'static Self {
        &ZERO_BLOCK_HASH
    }

    pub fn is_zero(&self) -> bool {
        self.value == [0u8; 32]
    }

    pub fn from_bytes(value: [u8; 32]) -> Self {
        Self { value }
    }

    pub const fn serialized_size() -> usize {
        32
    }

    pub fn serialize(&self, stream: &mut dyn Stream) -> Result<()> {
        stream.write_bytes(&self.value)
    }

    pub fn deserialize(stream: &mut impl Stream) -> Result<Self> {
        let mut result = Self::new();
        stream.read_bytes(&mut result.value, 32)?;
        Ok(result)
    }

    pub fn to_bytes(self) -> [u8; 32] {
        self.value
    }

    pub fn as_bytes(&'_ self) -> &'_ [u8; 32] {
        &self.value
    }

    pub fn encode_hex(&self) -> String {
        let mut result = String::with_capacity(64);
        for &byte in self.value.iter() {
            write!(&mut result, "{:02X}", byte).unwrap();
        }
        result
    }

    pub fn decode_hex(s: impl AsRef<str>) -> Result<BlockHash> {
        let mut bytes = [0u8; 32];
        hex::decode_to_slice(s.as_ref(), &mut bytes)?;
        Ok(BlockHash::from_bytes(bytes))
    }
}

impl From<u64> for BlockHash {
    fn from(value: u64) -> Self {
        let mut result = Self { value: [0; 32] };
        result.value[24..].copy_from_slice(&value.to_be_bytes());
        result
    }
}

impl Display for BlockHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode_upper(self.value))
    }
}

pub struct BlockHashBuilder {
    blake: blake2::VarBlake2b,
}

impl Default for BlockHashBuilder {
    fn default() -> Self {
        Self {
            blake: blake2::VarBlake2b::new_keyed(&[], 32),
        }
    }
}

impl BlockHashBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn update(mut self, data: impl AsRef<[u8]>) -> Self {
        self.blake.update(data);
        self
    }

    pub fn build(self) -> BlockHash {
        let mut hash_bytes = [0u8; 32];
        self.blake.finalize_variable(|result| {
            hash_bytes.copy_from_slice(result);
        });
        BlockHash::from_bytes(hash_bytes)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct Amount {
    value: u128, // native endian!
}

impl Amount {
    pub fn new(value: u128) -> Self {
        Self { value }
    }

    pub fn zero() -> Self {
        Self::new(0)
    }

    pub fn from_be_bytes(bytes: [u8; 16]) -> Self {
        Self {
            value: u128::from_be_bytes(bytes),
        }
    }

    pub const fn serialized_size() -> usize {
        std::mem::size_of::<u128>()
    }

    pub fn serialize(&self, stream: &mut dyn Stream) -> Result<()> {
        stream.write_bytes(&self.value.to_be_bytes())
    }

    pub fn deserialize(stream: &mut impl Stream) -> Result<Self> {
        let mut buffer = [0u8; 16];
        let len = buffer.len();
        stream.read_bytes(&mut buffer, len)?;
        Ok(Amount::new(u128::from_be_bytes(buffer)))
    }

    pub fn to_be_bytes(self) -> [u8; 16] {
        self.value.to_be_bytes()
    }

    pub fn encode_hex(&self) -> String {
        format!("{:032X}", self.value)
    }

    pub fn decode_hex(s: impl AsRef<str>) -> Result<Self> {
        let value = u128::from_str_radix(s.as_ref(), 16)?;
        Ok(Amount::new(value))
    }

    pub fn decode_dec(s: impl AsRef<str>) -> Result<Self> {
        Ok(Self::new(s.as_ref().parse::<u128>()?))
    }

    pub fn to_string_dec(self) -> String {
        self.value.to_string()
    }
}

impl From<u128> for Amount {
    fn from(value: u128) -> Self {
        Amount::new(value)
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Signature {
    bytes: [u8; 64],
}

impl Default for Signature {
    fn default() -> Self {
        Self { bytes: [0; 64] }
    }
}

impl Signature {
    pub fn new() -> Self {
        Self { bytes: [0u8; 64] }
    }

    pub fn from_bytes(bytes: [u8; 64]) -> Self {
        Self { bytes }
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
        Ok(Self::from_bytes(bytes.try_into()?))
    }

    pub const fn serialized_size() -> usize {
        64
    }

    pub fn serialize(&self, stream: &mut dyn Stream) -> Result<()> {
        stream.write_bytes(&self.bytes)
    }

    pub fn deserialize(stream: &mut impl Stream) -> Result<Signature> {
        let mut result = Signature { bytes: [0; 64] };

        stream.read_bytes(&mut result.bytes, 64)?;
        Ok(result)
    }

    pub fn as_bytes(&'_ self) -> &'_ [u8; 64] {
        &self.bytes
    }

    pub fn to_be_bytes(&self) -> [u8; 64] {
        self.bytes
    }

    #[cfg(test)]
    pub fn make_invalid(&mut self) {
        self.bytes[31] ^= 1;
    }

    pub fn encode_hex(&self) -> String {
        let mut result = String::with_capacity(128);
        for byte in self.bytes {
            write!(&mut result, "{:02X}", byte).unwrap();
        }
        result
    }

    pub fn decode_hex(s: impl AsRef<str>) -> Result<Self> {
        let mut bytes = [0u8; 64];
        hex::decode_to_slice(s.as_ref(), &mut bytes)?;
        Ok(Signature::from_bytes(bytes))
    }
}

#[derive(Clone, PartialEq, Eq, Default, Debug, Copy, Hash)]
pub struct HashOrAccount {
    bytes: [u8; 32],
}

impl HashOrAccount {
    pub fn new() -> Self {
        Self { bytes: [0u8; 32] }
    }

    pub fn is_zero(&self) -> bool {
        self.bytes == [0u8; 32]
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self { bytes }
    }

    pub const fn serialized_size() -> usize {
        32
    }

    pub fn serialize(&self, stream: &mut dyn Stream) -> Result<()> {
        stream.write_bytes(&self.bytes)
    }

    pub fn deserialize(stream: &mut impl Stream) -> Result<Self> {
        let mut result = Self::new();
        stream.read_bytes(&mut result.bytes, 32)?;
        Ok(result)
    }

    pub fn to_bytes(self) -> [u8; 32] {
        self.bytes
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.bytes
    }

    pub fn encode_hex(&self) -> String {
        let mut result = String::with_capacity(64);
        for byte in self.bytes {
            write!(&mut result, "{:02X}", byte).unwrap();
        }
        result
    }

    pub fn decode_hex(s: impl AsRef<str>) -> Result<Self> {
        let mut bytes = [0u8; 32];
        hex::decode_to_slice(s.as_ref(), &mut bytes)?;
        Ok(Self::from_bytes(bytes))
    }

    pub fn to_account(self) -> Account {
        Account::from_bytes(self.bytes)
    }
}

#[derive(Clone, PartialEq, Eq, Default, Debug, Copy)]
pub struct Link {
    inner: HashOrAccount,
}

impl Link {
    pub fn new() -> Self {
        Self {
            inner: HashOrAccount::new(),
        }
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self {
            inner: HashOrAccount::from_bytes(bytes),
        }
    }

    pub const fn serialized_size() -> usize {
        32
    }

    pub fn deserialize(stream: &mut impl Stream) -> Result<Self> {
        HashOrAccount::deserialize(stream).map(|inner| Self { inner })
    }

    pub fn decode_hex(s: impl AsRef<str>) -> Result<Self> {
        HashOrAccount::decode_hex(s).map(|inner| Self { inner })
    }

    pub fn is_zero(&self) -> bool {
        self.inner.is_zero()
    }
}

impl Deref for Link {
    type Target = HashOrAccount;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl From<u64> for HashOrAccount {
    fn from(value: u64) -> Self {
        let mut result = Self::new();
        result.bytes[24..].copy_from_slice(&value.to_be_bytes());
        result
    }
}

impl From<u64> for Link {
    fn from(value: u64) -> Self {
        Self {
            inner: HashOrAccount::from(value),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Default, Debug, Copy, Hash)]
pub struct Root {
    inner: HashOrAccount,
}

impl Root {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self {
            inner: HashOrAccount::from_bytes(bytes),
        }
    }
}

impl Deref for Root {
    type Target = HashOrAccount;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl From<u64> for Root {
    fn from(value: u64) -> Self {
        let mut bytes = [0; 32];
        bytes[..8].copy_from_slice(&value.to_le_bytes());
        Self::from_bytes(bytes)
    }
}

#[derive(Default)]
pub struct RawKey {
    bytes: [u8; 32],
}

impl RawKey {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self { bytes }
    }

    pub fn as_bytes(&'_ self) -> &'_ [u8; 32] {
        &self.bytes
    }

    pub fn encode_hex(&self) -> String {
        let mut result = String::with_capacity(64);
        for byte in self.bytes {
            write!(&mut result, "{:02X}", byte).unwrap();
        }
        result
    }
}

impl TryFrom<&RawKey> for PublicKey {
    type Error = anyhow::Error;
    fn try_from(prv: &RawKey) -> Result<Self, Self::Error> {
        let secret = ed25519_dalek_blake2b::SecretKey::from_bytes(prv.as_bytes())
            .map_err(|_| anyhow!("could not extract secret key"))?;
        let public = ed25519_dalek_blake2b::PublicKey::from(&secret);
        Ok(PublicKey {
            value: public.to_bytes(),
        })
    }
}

pub(crate) fn encode_hex(i: u128) -> String {
    let mut result = String::with_capacity(32);
    for byte in i.to_ne_bytes() {
        write!(&mut result, "{:02X}", byte).unwrap();
    }
    result
}

pub struct KeyPair {
    keypair: ed25519_dalek_blake2b::Keypair,
}

impl Default for KeyPair {
    fn default() -> Self {
        let mut rng = rand::thread_rng();
        let keypair = ed25519_dalek_blake2b::Keypair::generate(&mut rng);
        Self { keypair }
    }
}

impl KeyPair {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn zero() -> Self {
        Self::from_priv_key_bytes(&[0u8; 32]).unwrap()
    }

    pub fn from_priv_key_bytes(bytes: &[u8]) -> Result<Self> {
        let secret = ed25519_dalek_blake2b::SecretKey::from_bytes(bytes)
            .map_err(|_| anyhow!("could not load secret key"))?;
        let public = ed25519_dalek_blake2b::PublicKey::from(&secret);
        Ok(Self {
            keypair: ed25519_dalek_blake2b::Keypair { secret, public },
        })
    }

    pub fn from_priv_key_hex(s: impl AsRef<str>) -> Result<Self> {
        let mut bytes = [0u8; 32];
        hex::decode_to_slice(s.as_ref(), &mut bytes)?;
        Self::from_priv_key_bytes(&bytes)
    }

    pub fn public_key(&self) -> PublicKey {
        PublicKey::from_bytes(self.keypair.public.to_bytes())
    }

    pub fn private_key(&self) -> RawKey {
        RawKey::from_bytes(self.keypair.secret.to_bytes())
    }
}

pub fn sign_message(
    private_key: &RawKey,
    public_key: &PublicKey,
    data: &[u8],
) -> Result<Signature> {
    let secret = ed25519_dalek_blake2b::SecretKey::from_bytes(private_key.as_bytes())
        .map_err(|_| anyhow!("could not extract secret key"))?;
    let public = ed25519_dalek_blake2b::PublicKey::from_bytes(public_key.as_bytes())
        .map_err(|_| anyhow!("could not extract public key"))?;
    let expanded = ed25519_dalek_blake2b::ExpandedSecretKey::from(&secret);
    let signature = expanded.sign(data, &public);
    Ok(Signature::from_bytes(signature.to_bytes()))
}

pub fn validate_message(
    public_key: &PublicKey,
    message: &[u8],
    signature: &Signature,
) -> Result<()> {
    let public = ed25519_dalek_blake2b::PublicKey::from_bytes(public_key.as_bytes())
        .map_err(|_| anyhow!("could not extract public key"))?;
    let sig = ed25519_dalek_blake2b::Signature::from_bytes(&signature.to_be_bytes())
        .map_err(|_| anyhow!("invalid signature bytes"))?;
    public
        .verify_strict(message, &sig)
        .map_err(|_| anyhow!("could not verify message"))?;
    Ok(())
}

pub fn validate_message_batch(
    messages: &[Vec<u8>],
    public_keys: &[PublicKey],
    signatures: &[Signature],
    valid: &mut [i32],
) {
    let len = messages.len();
    assert!(public_keys.len() == len && signatures.len() == len && valid.len() == len);
    for i in 0..len {
        valid[i] = match validate_message(&public_keys[i], &messages[i], &signatures[i]) {
            Ok(_) => 1,
            Err(_) => 0,
        }
    }
}

pub fn to_string_hex(i: u64) -> String {
    format!("{:016X}", i)
}

pub fn from_string_hex(s: impl AsRef<str>) -> Result<u64> {
    let result = u64::from_str_radix(s.as_ref(), 16)?;
    Ok(result)
}

pub static XRB_RATIO: Lazy<u128> = Lazy::new(|| str::parse("1000000000000000000000000").unwrap()); // 10^24
pub static GXRB_RATIO: Lazy<u128> =
    Lazy::new(|| str::parse("1000000000000000000000000000000000").unwrap()); // 10^33

#[cfg(test)]
mod tests {
    use super::*;

    mod block_hash {
        use super::*;

        #[test]
        fn block_hash_encode_hex() {
            assert_eq!(
                BlockHash::new().encode_hex(),
                "0000000000000000000000000000000000000000000000000000000000000000"
            );
            assert_eq!(
                BlockHash::from(0x12ab).encode_hex(),
                "00000000000000000000000000000000000000000000000000000000000012AB"
            );
            assert_eq!(
                BlockHash::from_bytes([0xff; 32]).encode_hex(),
                "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF"
            );
        }
    }

    #[test]
    fn ed25519_signing() -> Result<()> {
        let secret_key = ed25519_dalek_blake2b::SecretKey::from_bytes(&[0u8; 32]).unwrap();
        let public_key = ed25519_dalek_blake2b::PublicKey::from(&secret_key);
        let message = [0u8; 32];
        let expanded_prv_key = ed25519_dalek_blake2b::ExpandedSecretKey::from(&secret_key);
        let signature = expanded_prv_key.sign(&message, &public_key);
        public_key.verify_strict(&message, &signature).unwrap();

        let mut sig_bytes = signature.to_bytes();
        sig_bytes[32] ^= 0x1;
        let signature = ed25519_dalek_blake2b::Signature::from_bytes(&sig_bytes).unwrap();
        assert!(public_key.verify_strict(&message, &signature).is_err());

        Ok(())
    }

    #[test]
    fn sign_message_test() -> Result<()> {
        let keypair = KeyPair::new();
        let data = [0u8; 32];
        let signature = sign_message(&keypair.private_key(), &keypair.public_key(), &data)?;
        validate_message(&keypair.public_key(), &data, &signature)?;
        Ok(())
    }
}
