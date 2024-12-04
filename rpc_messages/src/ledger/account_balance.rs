use crate::RpcBool;
use rsnano_core::{Account, Amount};
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AccountBalanceArgs {
    pub account: Account,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_only_confirmed: Option<RpcBool>,
}

impl AccountBalanceArgs {
    pub fn new(account: Account) -> AccountBalanceArgs {
        AccountBalanceArgs {
            account,
            include_only_confirmed: None,
        }
    }

    pub fn build(account: Account) -> AccountBalanceArgsBuilder {
        AccountBalanceArgsBuilder::new(account)
    }
}

impl From<Account> for AccountBalanceArgs {
    fn from(account: Account) -> Self {
        Self {
            account,
            include_only_confirmed: None,
        }
    }
}

pub struct AccountBalanceArgsBuilder {
    args: AccountBalanceArgs,
}

impl AccountBalanceArgsBuilder {
    fn new(account: Account) -> Self {
        Self {
            args: AccountBalanceArgs {
                account,
                include_only_confirmed: None,
            },
        }
    }

    pub fn include_unconfirmed_blocks(mut self) -> Self {
        self.args.include_only_confirmed = Some(false.into());
        self
    }

    pub fn finish(self) -> AccountBalanceArgs {
        self.args
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AccountBalanceResponse {
    pub balance: Amount,
    pub pending: Amount,
    pub receivable: Amount,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RpcCommand;
    use serde_json::to_string_pretty;

    #[test]
    fn serialize_account_balance_command_include_unconfirmed_blocks() {
        let account_balance_args = AccountBalanceArgsBuilder::new(Account::zero())
            .include_unconfirmed_blocks()
            .finish();
        assert_eq!(
            to_string_pretty(&RpcCommand::AccountBalance(account_balance_args)).unwrap(),
            r#"{
  "action": "account_balance",
  "account": "nano_1111111111111111111111111111111111111111111111111111hifc8npp",
  "include_only_confirmed": "false"
}"#
        )
    }

    #[test]
    fn deserialize_account_balance_command_include_unconfirmed_blocks() {
        let json = r#"{
            "action": "account_balance",
            "account": "nano_111111111111111111111111111111111111111111111111115uwdgas549",
            "include_only_confirmed": "true"
        }"#;

        let deserialized: RpcCommand = serde_json::from_str(json).unwrap();

        if let RpcCommand::AccountBalance(args) = deserialized {
            assert_eq!(args.account, Account::from(123));
            assert_eq!(args.include_only_confirmed, Some(true.into()));
        } else {
            panic!("Deserialized to wrong RpcCommand variant");
        }
    }

    #[test]
    fn serialize_account_balance_command_default() {
        let account_balance_args = AccountBalanceArgsBuilder::new(Account::zero()).finish();
        assert_eq!(
            to_string_pretty(&RpcCommand::AccountBalance(account_balance_args)).unwrap(),
            r#"{
  "action": "account_balance",
  "account": "nano_1111111111111111111111111111111111111111111111111111hifc8npp"
}"#
        )
    }

    #[test]
    fn deserialize_account_balance_command_default() {
        let account_balance_args = AccountBalanceArgsBuilder::new(Account::zero()).finish();
        let cmd = RpcCommand::AccountBalance(account_balance_args);
        let serialized = serde_json::to_string_pretty(&cmd).unwrap();
        let deserialized: RpcCommand = serde_json::from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized)
    }

    #[test]
    fn account_balance_args_from_account() {
        let account = Account::from(123);
        let args: AccountBalanceArgs = account.into();

        assert_eq!(args.account, account);
        assert_eq!(args.include_only_confirmed, None);
    }

    #[test]
    fn serialize_account_balance_dto() {
        let account_balance = AccountBalanceResponse {
            balance: Amount::raw(1000),
            pending: Amount::raw(200),
            receivable: Amount::raw(300),
        };

        let serialized = serde_json::to_string(&account_balance).unwrap();

        assert_eq!(
            serialized,
            r#"{"balance":"1000","pending":"200","receivable":"300"}"#
        );
    }

    #[test]
    fn deserialize_account_balance_dto() {
        let json_str = r#"{"balance":"1000","pending":"200","receivable":"300"}"#;

        let deserialized: AccountBalanceResponse = serde_json::from_str(json_str).unwrap();

        let expected = AccountBalanceResponse {
            balance: Amount::raw(1000),
            pending: Amount::raw(200),
            receivable: Amount::raw(300),
        };

        assert_eq!(deserialized, expected);
    }
}
