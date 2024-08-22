use super::{PublicKey, RawKey, Signature};
use crate::{Account, Block, StateBlock};
use anyhow::Context;
use rsnano_nullable_random::NullableRng;

pub struct KeyPair {
    keypair: ed25519_dalek_blake2b::Keypair,
}

pub struct KeyPairFactory {
    rng: NullableRng,
}

impl KeyPairFactory {
    #[allow(dead_code)]
    fn new(rng: NullableRng) -> Self {
        Self { rng }
    }

    pub fn new_null() -> Self {
        Self {
            rng: NullableRng::new_null(),
        }
    }

    pub fn new_null_with(prv: RawKey) -> Self {
        Self {
            rng: NullableRng::new_null_bytes(prv.as_bytes()),
        }
    }

    pub fn create_key_pair(&mut self) -> KeyPair {
        let keypair = ed25519_dalek_blake2b::Keypair::generate(&mut self.rng);
        KeyPair { keypair }
    }
}

impl Default for KeyPairFactory {
    fn default() -> Self {
        Self {
            rng: NullableRng::thread_rng(),
        }
    }
}

impl Default for KeyPair {
    fn default() -> Self {
        KeyPairFactory::default().create_key_pair()
    }
}

impl Clone for KeyPair {
    fn clone(&self) -> Self {
        Self::from_priv_key_bytes(self.keypair.secret.as_bytes()).unwrap()
    }
}

impl KeyPair {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn zero() -> Self {
        Self::from_priv_key_bytes(&[0u8; 32]).unwrap()
    }

    pub fn from_priv_key_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        let secret = ed25519_dalek_blake2b::SecretKey::from_bytes(bytes)
            .map_err(|_| anyhow!("could not load secret key"))?;
        let public = ed25519_dalek_blake2b::PublicKey::from(&secret);
        Ok(Self {
            keypair: ed25519_dalek_blake2b::Keypair { secret, public },
        })
    }

    pub fn from_priv_key_hex(s: impl AsRef<str>) -> anyhow::Result<Self> {
        let input = s.as_ref();
        let mut bytes = [0u8; 32];
        hex::decode_to_slice(input, &mut bytes)
            .with_context(|| format!("input string: '{}'", input))?;
        Self::from_priv_key_bytes(&bytes)
    }

    pub fn account(&self) -> Account {
        Account::from_bytes(self.keypair.public.to_bytes())
    }

    pub fn public_key(&self) -> PublicKey {
        PublicKey::from_bytes(self.keypair.public.to_bytes())
    }

    pub fn private_key(&self) -> RawKey {
        RawKey::from_bytes(self.keypair.secret.to_bytes())
    }
}

impl From<u64> for KeyPair {
    fn from(value: u64) -> Self {
        let mut bytes = [0; 32];
        bytes[..8].copy_from_slice(&value.to_be_bytes());
        Self::from_priv_key_bytes(&bytes).unwrap()
    }
}

impl From<RawKey> for KeyPair {
    fn from(value: RawKey) -> Self {
        Self::from_priv_key_bytes(value.as_bytes()).unwrap()
    }
}

pub fn sign_message(private_key: &RawKey, public_key: &PublicKey, data: &[u8]) -> Signature {
    let secret = ed25519_dalek_blake2b::SecretKey::from_bytes(private_key.as_bytes())
        .expect("could not extract secret key");
    let public = ed25519_dalek_blake2b::PublicKey::from_bytes(public_key.as_bytes())
        .expect("could not extract public key");
    let expanded = ed25519_dalek_blake2b::ExpandedSecretKey::from(&secret);
    let signature = expanded.sign(data, &public);
    Signature::from_bytes(signature.to_bytes())
}

pub fn validate_message(
    public_key: &PublicKey,
    message: &[u8],
    signature: &Signature,
) -> anyhow::Result<()> {
    let public = ed25519_dalek_blake2b::PublicKey::from_bytes(public_key.as_bytes())
        .map_err(|_| anyhow!("could not extract public key"))?;
    let sig = ed25519_dalek_blake2b::Signature::from_bytes(signature.as_bytes())
        .map_err(|_| anyhow!("invalid signature bytes"))?;
    public
        .verify_strict(message, &sig)
        .map_err(|_| anyhow!("could not verify message"))?;
    Ok(())
}

pub fn validate_block_signature(block: &StateBlock) -> anyhow::Result<()> {
    validate_message(
        &block.account().into(),
        block.hash().as_bytes(),
        block.block_signature(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ed25519_signing() -> anyhow::Result<()> {
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
    fn sign_message_test() -> anyhow::Result<()> {
        let keypair = KeyPair::new();
        let data = [0u8; 32];
        let signature = sign_message(&keypair.private_key(), &keypair.public_key(), &data);
        validate_message(&keypair.public_key(), &data, &signature)?;
        Ok(())
    }

    #[test]
    fn signing_same_message_twice_produces_equal_signatures() {
        // the C++ implementation adds random bytes and a padding when signing for extra security and for making side channel attacks more difficult.
        // Currently the Rust impl does not do that.
        // In C++ signing the same message twice will produce different signatures. In Rust we get the same signature.
        let keypair = KeyPair::new();
        let data = [1, 2, 3];
        let signature_a = sign_message(&keypair.private_key(), &keypair.public_key(), &data);
        let signature_b = sign_message(&keypair.private_key(), &keypair.public_key(), &data);
        assert_eq!(signature_a, signature_b);
    }

    mod key_pair_factory {
        use super::*;

        #[test]
        fn create_key_pair() {
            let random_data = [
                0x11, 0x22, 0x33, 0x44, 0x11, 0x22, 0x33, 0x44, 0x11, 0x22, 0x33, 0x44, 0x11, 0x22,
                0x33, 0x44, 0x11, 0x22, 0x33, 0x44, 0x11, 0x22, 0x33, 0x44, 0x11, 0x22, 0x33, 0x44,
                0x11, 0x22, 0x33, 0x44,
            ];
            let rng = NullableRng::new_null_bytes(&random_data);
            let mut key_pair_factory = KeyPairFactory::new(rng);

            let key_pair = key_pair_factory.create_key_pair();

            assert_eq!(key_pair.private_key().as_bytes(), &random_data);
        }

        #[test]
        fn nullable() {
            let mut key_pair_factory = KeyPairFactory::new_null();
            let key_pair = key_pair_factory.create_key_pair();
            assert_ne!(key_pair.private_key(), RawKey::zero());
        }

        #[test]
        fn configured_response() {
            let expected = RawKey::from_bytes([3; 32]);
            let mut key_pair_factory = KeyPairFactory::new_null_with(expected);
            assert_eq!(key_pair_factory.create_key_pair().private_key(), expected);
        }
    }
}
