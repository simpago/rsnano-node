use rsnano_core::{Account, JsonBlock, WalletId};
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ReceiveArgs {
    pub wallet: WalletId,
    pub account: Account,
    pub block: JsonBlock,
}
