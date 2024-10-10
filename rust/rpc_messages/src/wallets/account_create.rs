use crate::RpcCommand;
use rsnano_core::WalletId;
use serde::{Deserialize, Serialize};

impl RpcCommand {
    pub fn account_create(wallet: WalletId, index: Option<u32>, work: Option<bool>) -> Self {
        Self::AccountCreate(AccountCreateArgs {
            wallet,
            index,
            work,
        })
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AccountCreateArgs {
    pub wallet: WalletId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work: Option<bool>,
}

#[cfg(test)]
mod tests {
    use crate::RpcCommand;
    use serde_json::to_string_pretty;

    #[test]
    fn serialize_account_create_command_options_none() {
        assert_eq!(
            to_string_pretty(&RpcCommand::account_create(1.into(), None, None)).unwrap(),
            r#"{
  "action": "account_create",
  "wallet": "0000000000000000000000000000000000000000000000000000000000000001"
}"#
        )
    }

    #[test]
    fn serialize_account_create_command_optionss_some() {
        assert_eq!(
            to_string_pretty(&RpcCommand::account_create(1.into(), Some(1), Some(true))).unwrap(),
            r#"{
  "action": "account_create",
  "wallet": "0000000000000000000000000000000000000000000000000000000000000001",
  "index": 1,
  "work": true
}"#
        )
    }

    #[test]
    fn deserialize_account_create_command_options_none() {
        let cmd = RpcCommand::account_create(1.into(), None, None);
        let serialized = serde_json::to_string_pretty(&cmd).unwrap();
        let deserialized: RpcCommand = serde_json::from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized)
    }

    #[test]
    fn deserialize_account_create_command_options_some() {
        let cmd = RpcCommand::account_create(1.into(), Some(1), Some(true));
        let serialized = serde_json::to_string_pretty(&cmd).unwrap();
        let deserialized: RpcCommand = serde_json::from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized)
    }
}
