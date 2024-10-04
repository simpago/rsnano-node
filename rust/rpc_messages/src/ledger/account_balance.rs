use crate::RpcCommand;
use rsnano_core::Account;
use serde::{Deserialize, Serialize};

impl RpcCommand {
    pub fn account_balance(account: Account, include_only_confirmed: Option<bool>) -> Self {
        Self::AccountBalance(AccountBalanceArgs {
            account,
            include_only_confirmed,
        })
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AccountBalanceArgs {
    pub account: Account,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_only_confirmed: Option<bool>,
}

impl AccountBalanceArgs {
    pub fn new(account: Account, include_only_confirmed: Option<bool>) -> Self {
        Self {
            account,
            include_only_confirmed,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::RpcCommand;
    use rsnano_core::Account;
    use serde_json::to_string_pretty;

    #[test]
    fn serialize_account_balance_command_include_only_confirmed_none() {
        assert_eq!(
            to_string_pretty(&RpcCommand::account_balance(Account::zero(), None)).unwrap(),
            r#"{
  "action": "account_balance",
  "account": "nano_1111111111111111111111111111111111111111111111111111hifc8npp"
}"#
        )
    }

    #[test]
    fn serialize_account_balance_command_include_only_confirmed_some() {
        assert_eq!(
            to_string_pretty(&RpcCommand::account_balance(Account::zero(), Some(true))).unwrap(),
            r#"{
  "action": "account_balance",
  "account": "nano_1111111111111111111111111111111111111111111111111111hifc8npp",
  "include_only_confirmed": true
}"#
        )
    }

    #[test]
    fn deserialize_account_balance_command_include_only_confirmed_none() {
        let account = Account::from(123);
        let cmd = RpcCommand::account_balance(account, None);
        let serialized = serde_json::to_string_pretty(&cmd).unwrap();
        let deserialized: RpcCommand = serde_json::from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized)
    }

    #[test]
    fn deserialize_account_balance_command_include_only_confirmed_some() {
        let account = Account::from(123);
        let cmd = RpcCommand::account_balance(account, Some(true));
        let serialized = serde_json::to_string_pretty(&cmd).unwrap();
        let deserialized: RpcCommand = serde_json::from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized)
    }
}
