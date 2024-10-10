use crate::RpcCommand;
use rsnano_core::{RawKey, WalletId};
use serde::{Deserialize, Serialize};

impl RpcCommand {
    pub fn wallet_add(wallet_id: WalletId, key: RawKey, work: Option<bool>) -> Self {
        Self::WalletAdd(WalletAddArgs {
            wallet: wallet_id,
            key,
            work,
        })
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct WalletAddArgs {
    pub wallet: WalletId,
    pub key: RawKey,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work: Option<bool>,
}

impl WalletAddArgs {
    pub fn new(wallet: WalletId, key: RawKey, work: Option<bool>) -> Self {
        Self { wallet, key, work }
    }
}

#[cfg(test)]
mod tests {
    use crate::RpcCommand;
    use serde_json::to_string_pretty;

    #[test]
    fn serialize_wallet_add_command_work_none() {
        assert_eq!(
            to_string_pretty(&RpcCommand::wallet_add(1.into(), 2.into(), None)).unwrap(),
            r#"{
  "action": "wallet_add",
  "wallet": "0000000000000000000000000000000000000000000000000000000000000001",
  "key": "0000000000000000000000000000000000000000000000000000000000000002"
}"#
        )
    }

    #[test]
    fn serialize_wallet_add_command_work_some() {
        assert_eq!(
            to_string_pretty(&RpcCommand::wallet_add(1.into(), 2.into(), Some(true))).unwrap(),
            r#"{
  "action": "wallet_add",
  "wallet": "0000000000000000000000000000000000000000000000000000000000000001",
  "key": "0000000000000000000000000000000000000000000000000000000000000002",
  "work": true
}"#
        )
    }

    #[test]
    fn deserialize_wallet_add_command_work_none() {
        let cmd = RpcCommand::wallet_add(1.into(), 2.into(), None);
        let serialized = serde_json::to_string_pretty(&cmd).unwrap();
        let deserialized: RpcCommand = serde_json::from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized)
    }

    #[test]
    fn deserialize_wallet_add_command_work_some() {
        let cmd = RpcCommand::wallet_add(1.into(), 2.into(), Some(true));
        let serialized = serde_json::to_string_pretty(&cmd).unwrap();
        let deserialized: RpcCommand = serde_json::from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized)
    }
}
