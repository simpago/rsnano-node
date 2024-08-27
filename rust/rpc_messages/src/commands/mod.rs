mod ledger;
mod node;
mod utils;
mod wallets;

pub use ledger::*;
pub use node::*;
use serde::{Deserialize, Serialize};
pub use utils::*;
pub use wallets::*;

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum RpcCommand {
    AccountInfo(AccountInfoArgs),
    Keepalive(KeepaliveArgs),
    Stop,
    KeyCreate,
    Receive(ReceiveArgs),
    Send(SendArgs),
    WalletAdd(WalletAddArgs),
    WalletCreate,
}
