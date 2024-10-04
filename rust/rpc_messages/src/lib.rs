mod common;
mod ledger;
mod node;
mod utils;
mod wallets;

pub use common::*;
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
    AccountCreate(AccountCreateArgs),
    AccountBalance(AccountBalanceArgs),
}

#[cfg(test)]
mod tests {
    use crate::RpcCommand;
    use serde_json::to_string_pretty;

    #[test]
    fn serialize_stop_command() {
        assert_eq!(
            to_string_pretty(&RpcCommand::Stop).unwrap(),
            r#"{
  "action": "stop"
}"#
        )
    }
}
