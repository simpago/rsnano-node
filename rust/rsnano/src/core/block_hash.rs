use blake2::digest::Update;
use blake2::digest::VariableOutput;
use rand::thread_rng;
use rand::Rng;

use crate::u256_struct;

use super::Account;

u256_struct!(BlockHash);

impl BlockHash {
    pub fn random() -> Self {
        BlockHash::from_bytes(thread_rng().gen())
    }
}

impl From<&Account> for BlockHash {
    fn from(account: &Account) -> Self {
        Self::from_bytes(*account.as_bytes())
    }
}

impl From<Account> for BlockHash {
    fn from(account: Account) -> Self {
        Self::from_bytes(*account.as_bytes())
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
