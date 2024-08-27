mod receive;
mod send;
mod wallet_add;

use super::RpcCommand;
pub use receive::*;
use rsnano_core::{RawKey, WalletId};
pub use send::*;
pub use wallet_add::*;

impl RpcCommand {
    pub fn wallet_add(wallet_id: WalletId, key: RawKey) -> Self {
        Self::WalletAdd(WalletAddArgs {
            wallet: wallet_id,
            key,
        })
    }
}
