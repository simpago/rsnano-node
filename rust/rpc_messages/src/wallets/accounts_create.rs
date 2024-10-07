use crate::{RpcCommand, WalletWithCountArgs};
use rsnano_core::WalletId;
use serde::{Deserialize, Serialize};

impl RpcCommand {
    pub fn accounts_create(wallet: WalletId, count: u64, work: Option<bool>) -> Self {
        Self::AccountsCreate(AccountsCreateArgs::new(wallet, count, work))
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AccountsCreateArgs {
    #[serde(flatten)]
    pub wallet_with_count: WalletWithCountArgs,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work: Option<bool>,
}

impl AccountsCreateArgs {
    pub fn new(wallet: WalletId, count: u64, work: Option<bool>) -> Self {
        Self {
            wallet_with_count: WalletWithCountArgs::new(wallet, count),
            work,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::RpcCommand;
    use serde_json::to_string_pretty;

    #[test]
    fn serialize_accounts_create_command_work_some() {
        assert_eq!(
            to_string_pretty(&RpcCommand::accounts_create(1.into(), 2, Some(true))).unwrap(),
            r#"{
  "action": "accounts_create",
  "wallet": "0000000000000000000000000000000000000000000000000000000000000001",
  "count": 2,
  "work": true
}"#
        )
    }

    #[test]
    fn serialize_accounts_create_command_work_none() {
        assert_eq!(
            to_string_pretty(&RpcCommand::accounts_create(2.into(), 3, None)).unwrap(),
            r#"{
  "action": "accounts_create",
  "wallet": "0000000000000000000000000000000000000000000000000000000000000002",
  "count": 3
}"#
        )
    }

    #[test]
    fn deserialize_accounts_create_command_work_none() {
        let cmd = RpcCommand::accounts_create(1.into(), 2, None);
        let serialized = serde_json::to_string_pretty(&cmd).unwrap();
        let deserialized: RpcCommand = serde_json::from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized)
    }

    #[test]
    fn deserialize_accounts_create_command_work_some() {
        let cmd = RpcCommand::accounts_create(4.into(), 5, Some(true));
        let serialized = serde_json::to_string(&cmd).unwrap();
        let deserialized: RpcCommand = serde_json::from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized)
    }
}
