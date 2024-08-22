use rsnano_core::{Account, Amount, BlockHash, PublicKey, RawKey};
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AccountInfoDto {
    pub frontier: BlockHash,
    pub block_count: u64,
    pub balance: Amount,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyPairDto {
    pub private_key: RawKey,
    pub public_key: PublicKey,
    pub as_string: Account,
}
