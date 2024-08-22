use rsnano_core::{Account, Amount, JsonBlock, RawKey, WalletId};
use serde::{Deserialize, Serialize};
use std::net::Ipv6Addr;

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum RpcCommand {
    AccountInfo(AccountInfoCmd),
    WalletAdd(WalletAddCmd),
    Receive(ReceiveCmd),
    Send(SendCmd),
    Keepalive(KeepaliveCmd),
    KeyCreate,
    WalletCreate,
    Stop,
}

impl RpcCommand {
    pub fn account_info(account: Account) -> Self {
        Self::AccountInfo(AccountInfoCmd { account })
    }

    pub fn wallet_add(wallet_id: WalletId, key: RawKey) -> Self {
        Self::WalletAdd(WalletAddCmd {
            wallet: wallet_id,
            key,
        })
    }

    pub fn keepalive(address: Ipv6Addr, port: u16) -> Self {
        Self::Keepalive(KeepaliveCmd { address, port })
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AccountInfoCmd {
    pub account: Account,
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct WalletAddCmd {
    pub wallet: WalletId,
    pub key: RawKey,
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ReceiveCmd {
    pub wallet: WalletId,
    pub account: Account,
    pub block: JsonBlock,
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct SendCmd {
    pub wallet: WalletId,
    pub source: Account,
    pub destination: Account,
    pub amount: Amount,
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct KeepaliveCmd {
    pub address: Ipv6Addr,
    pub port: u16,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize() {
        let account = Account::from(123);
        let cmd = RpcCommand::account_info(account);
        let serialized = serde_json::to_string_pretty(&cmd).unwrap();
        let deserialized: RpcCommand = serde_json::from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized)
    }

    #[test]
    fn serialize_account_info_command() {
        assert_eq!(
            serde_json::to_string_pretty(&RpcCommand::account_info(Account::from(123))).unwrap(),
            r#"{
  "action": "account_info",
  "account": "nano_111111111111111111111111111111111111111111111111115uwdgas549"
}"#
        )
    }

    #[test]
    fn serialize_stop_command() {
        assert_eq!(
            serde_json::to_string_pretty(&RpcCommand::Stop).unwrap(),
            r#"{
  "action": "stop"
}"#
        )
    }

    #[test]
    fn serialize_wallet_add_command() {
        assert_eq!(
            serde_json::to_string_pretty(&RpcCommand::wallet_add(1.into(), 2.into())).unwrap(),
            r#"{
  "action": "wallet_add",
  "wallet": "0000000000000000000000000000000000000000000000000000000000000001",
  "key": "0000000000000000000000000000000000000000000000000000000000000002"
}"#
        )
    }
}
