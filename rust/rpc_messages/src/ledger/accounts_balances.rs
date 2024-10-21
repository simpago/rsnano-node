use crate::RpcCommand;
use rsnano_core::Account;
use serde::{Deserialize, Serialize};

impl RpcCommand {
    pub fn accounts_balances(args: AccountsBalancesArgs) -> Self {
        RpcCommand::AccountsBalances(args)
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AccountsBalancesArgs {
    pub accounts: Vec<Account>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_only_confirmed: Option<bool>,
}

impl AccountsBalancesArgs {
    pub fn new(accounts: Vec<Account>) -> AccountsBalancesArgsBuilder {
        AccountsBalancesArgsBuilder::new(accounts)
    }
}

pub struct AccountsBalancesArgsBuilder {
    args: AccountsBalancesArgs,
}

impl AccountsBalancesArgsBuilder {
    fn new(accounts: Vec<Account>) -> Self {
        Self {
            args: AccountsBalancesArgs {
                accounts,
                include_only_confirmed: None,
            },
        }
    }

    pub fn include_unconfirmed_blocks(mut self) -> Self {
        self.args.include_only_confirmed = Some(false);
        self
    }

    pub fn build(self) -> AccountsBalancesArgs {
        self.args
    }
}

impl From<Vec<Account>> for AccountsBalancesArgs {
    fn from(accounts: Vec<Account>) -> Self {
        Self {
            accounts,
            include_only_confirmed: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsnano_core::Account;

    #[test]
    fn deserialize_accounts_balances_rpc_command() {
        let accounts = vec![
            Account::decode_account(
                "nano_3t6k35gi95xu6tergt6p69ck76ogmitsa8mnijtpxm9fkcm736xtoncuohr3",
            )
            .unwrap(),
            Account::decode_account(
                "nano_3i1aq1cchnmbn9x5rsbap8b15akfh7wj7pwskuzi7ahz8oq6cobd99d4r3b7",
            )
            .unwrap(),
        ];

        let command = RpcCommand::accounts_balances(accounts.clone().into());
        let serialized = serde_json::to_string(&command).unwrap();
        let expected = r#"{"action":"accounts_balances","accounts":["nano_3t6k35gi95xu6tergt6p69ck76ogmitsa8mnijtpxm9fkcm736xtoncuohr3","nano_3i1aq1cchnmbn9x5rsbap8b15akfh7wj7pwskuzi7ahz8oq6cobd99d4r3b7"]}"#;
        assert_eq!(serialized, expected);

        let args = AccountsBalancesArgsBuilder::new(accounts)
            .include_unconfirmed_blocks()
            .build();
        let command = RpcCommand::accounts_balances(args);
        let serialized = serde_json::to_string(&command).unwrap();
        let expected = r#"{"action":"accounts_balances","accounts":["nano_3t6k35gi95xu6tergt6p69ck76ogmitsa8mnijtpxm9fkcm736xtoncuohr3","nano_3i1aq1cchnmbn9x5rsbap8b15akfh7wj7pwskuzi7ahz8oq6cobd99d4r3b7"],"include_only_confirmed":false}"#;
        assert_eq!(serialized, expected);
    }

    #[test]
    fn serialize_accounts_balances_rpc_command() {
        let json_data = r#"
        {
            "action": "accounts_balances",
            "accounts": [
                "nano_3t6k35gi95xu6tergt6p69ck76ogmitsa8mnijtpxm9fkcm736xtoncuohr3",
                "nano_3i1aq1cchnmbn9x5rsbap8b15akfh7wj7pwskuzi7ahz8oq6cobd99d4r3b7"
            ],
            "include_only_confirmed": true
        }"#;

        let deserialized: RpcCommand = serde_json::from_str(json_data).unwrap();

        match deserialized {
            RpcCommand::AccountsBalances(args) => {
                assert_eq!(args.accounts.len(), 2);
                assert_eq!(
                    args.accounts[0],
                    Account::decode_account(
                        "nano_3t6k35gi95xu6tergt6p69ck76ogmitsa8mnijtpxm9fkcm736xtoncuohr3"
                    )
                    .unwrap()
                );
                assert_eq!(
                    args.accounts[1],
                    Account::decode_account(
                        "nano_3i1aq1cchnmbn9x5rsbap8b15akfh7wj7pwskuzi7ahz8oq6cobd99d4r3b7"
                    )
                    .unwrap()
                );
                assert_eq!(args.include_only_confirmed, Some(true));
            }
            _ => panic!("Deserialized to wrong RpcCommand variant"),
        }

        let json_data_without_option = r#"
        {
            "action": "accounts_balances",
            "accounts": [
                "nano_3t6k35gi95xu6tergt6p69ck76ogmitsa8mnijtpxm9fkcm736xtoncuohr3",
                "nano_3i1aq1cchnmbn9x5rsbap8b15akfh7wj7pwskuzi7ahz8oq6cobd99d4r3b7"
            ]
        }"#;

        let deserialized: RpcCommand = serde_json::from_str(json_data_without_option).unwrap();

        match deserialized {
            RpcCommand::AccountsBalances(args) => {
                assert_eq!(args.accounts.len(), 2);
                assert_eq!(args.include_only_confirmed, None);
            }
            _ => panic!("Deserialized to wrong RpcCommand variant"),
        }
    }

    #[test]
    fn test_accounts_balances_args_builder() {
        let accounts = vec![
            Account::decode_account(
                "nano_3t6k35gi95xu6tergt6p69ck76ogmitsa8mnijtpxm9fkcm736xtoncuohr3",
            )
            .unwrap(),
            Account::decode_account(
                "nano_3i1aq1cchnmbn9x5rsbap8b15akfh7wj7pwskuzi7ahz8oq6cobd99d4r3b7",
            )
            .unwrap(),
        ];

        let args = AccountsBalancesArgs::new(accounts.clone())
            .include_unconfirmed_blocks()
            .build();

        assert_eq!(args.accounts, accounts);
        assert_eq!(args.include_only_confirmed, Some(false));

        let args_default = AccountsBalancesArgs::new(accounts.clone()).build();

        assert_eq!(args_default.accounts, accounts);
        assert_eq!(args_default.include_only_confirmed, None);
    }
}
