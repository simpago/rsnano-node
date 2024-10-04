use rsnano_core::{Account, PublicKey, RawKey};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyPairDto {
    pub private: RawKey,
    pub public: PublicKey,
    pub account: Account,
}
