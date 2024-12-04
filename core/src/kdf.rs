use crate::RawKey;
use argon2::{Variant, Version};

/// Key derivation function
#[derive(Clone)]
pub struct KeyDerivationFunction {
    kdf_work: u32,
}

impl KeyDerivationFunction {
    pub fn new(kdf_work: u32) -> Self {
        Self { kdf_work }
    }

    pub fn hash_password(&self, password: &str, salt: &[u8; 32]) -> RawKey {
        let config = argon2::Config {
            hash_length: 32,
            lanes: 1,
            mem_cost: self.kdf_work,
            time_cost: 1,
            variant: Variant::Argon2d,
            version: Version::Version10,
            ..Default::default()
        };

        let hash = argon2::hash_raw(password.as_bytes(), salt, &config).unwrap();
        RawKey::from_bytes(hash.as_slice().try_into().unwrap())
    }
}
