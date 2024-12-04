use crate::{AccountBalanceResponse, RpcBool};
use rsnano_core::Account;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AccountsBalancesArgs {
    pub accounts: Vec<Account>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_only_confirmed: Option<RpcBool>,
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
        self.args.include_only_confirmed = Some(false.into());
        self
    }

    pub fn finish(self) -> AccountsBalancesArgs {
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

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AccountsBalancesResponse {
    pub balances: HashMap<Account, AccountBalanceResponse>,
}

#[cfg(test)]
mod tests {
    use crate::RpcCommand;

    use super::*;
    use rsnano_core::Amount;

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

        let command = RpcCommand::AccountsBalances(accounts.clone().into());
        let serialized = serde_json::to_string(&command).unwrap();
        let expected = r#"{"action":"accounts_balances","accounts":["nano_3t6k35gi95xu6tergt6p69ck76ogmitsa8mnijtpxm9fkcm736xtoncuohr3","nano_3i1aq1cchnmbn9x5rsbap8b15akfh7wj7pwskuzi7ahz8oq6cobd99d4r3b7"]}"#;
        assert_eq!(serialized, expected);

        let args = AccountsBalancesArgsBuilder::new(accounts)
            .include_unconfirmed_blocks()
            .finish();
        let command = RpcCommand::AccountsBalances(args);
        let serialized = serde_json::to_string(&command).unwrap();
        let expected = r#"{"action":"accounts_balances","accounts":["nano_3t6k35gi95xu6tergt6p69ck76ogmitsa8mnijtpxm9fkcm736xtoncuohr3","nano_3i1aq1cchnmbn9x5rsbap8b15akfh7wj7pwskuzi7ahz8oq6cobd99d4r3b7"],"include_only_confirmed":"false"}"#;
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
            "include_only_confirmed": "true"
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
                assert_eq!(args.include_only_confirmed, Some(true.into()));
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
            .finish();

        assert_eq!(args.accounts, accounts);
        assert_eq!(args.include_only_confirmed, Some(false.into()));

        let args_default = AccountsBalancesArgs::new(accounts.clone()).finish();

        assert_eq!(args_default.accounts, accounts);
        assert_eq!(args_default.include_only_confirmed, None);
    }

    #[test]
    fn serialize_accounts_balances_dto() {
        let mut balances = HashMap::new();
        balances.insert(
            Account::decode_account(
                "nano_3t6k35gi95xu6tergt6p69ck76ogmitsa8mnijtpxm9fkcm736xtoncuohr3",
            )
            .unwrap(),
            AccountBalanceResponse {
                balance: Amount::raw(325586539664609129644855132177),
                pending: Amount::raw(2309372032769300000000000000000000),
                receivable: Amount::raw(2309372032769300000000000000000000),
            },
        );
        balances.insert(
            Account::decode_account(
                "nano_3i1aq1cchnmbn9x5rsbap8b15akfh7wj7pwskuzi7ahz8oq6cobd99d4r3b7",
            )
            .unwrap(),
            AccountBalanceResponse {
                balance: Amount::raw(10000000),
                pending: Amount::raw(0),
                receivable: Amount::raw(0),
            },
        );

        let dto = AccountsBalancesResponse { balances };

        let serialized = serde_json::to_string(&dto).unwrap();
        let deserialized: AccountsBalancesResponse = serde_json::from_str(&serialized).unwrap();

        assert_eq!(dto.balances.len(), deserialized.balances.len());
        assert_eq!(
            dto.balances
                .get(
                    &Account::decode_account(
                        "nano_3t6k35gi95xu6tergt6p69ck76ogmitsa8mnijtpxm9fkcm736xtoncuohr3"
                    )
                    .unwrap()
                )
                .unwrap()
                .balance,
            Amount::raw(325586539664609129644855132177)
        );
        assert_eq!(
            dto.balances
                .get(
                    &Account::decode_account(
                        "nano_3i1aq1cchnmbn9x5rsbap8b15akfh7wj7pwskuzi7ahz8oq6cobd99d4r3b7"
                    )
                    .unwrap()
                )
                .unwrap()
                .pending,
            Amount::raw(0)
        );
    }

    #[test]
    fn deserialize_accounts_balances_dto() {
        let json_data = r#"
        {
            "balances": {
                "nano_3t6k35gi95xu6tergt6p69ck76ogmitsa8mnijtpxm9fkcm736xtoncuohr3": {
                    "balance": "325586539664609129644855132177",
                    "pending": "2309372032769300000000000000000000",
                    "receivable": "2309372032769300000000000000000000"
                },
                "nano_3i1aq1cchnmbn9x5rsbap8b15akfh7wj7pwskuzi7ahz8oq6cobd99d4r3b7": {
                    "balance": "10000000",
                    "pending": "0",
                    "receivable": "0"
                }
            }
        }"#;

        let deserialized: AccountsBalancesResponse = serde_json::from_str(json_data).unwrap();

        assert_eq!(deserialized.balances.len(), 2);

        let balance1 = deserialized
            .balances
            .get(
                &Account::decode_account(
                    "nano_3t6k35gi95xu6tergt6p69ck76ogmitsa8mnijtpxm9fkcm736xtoncuohr3",
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(
            balance1.balance,
            Amount::raw(325586539664609129644855132177)
        );
        assert_eq!(
            balance1.pending,
            Amount::raw(2309372032769300000000000000000000)
        );
        assert_eq!(
            balance1.receivable,
            Amount::raw(2309372032769300000000000000000000)
        );

        let balance2 = deserialized
            .balances
            .get(
                &Account::decode_account(
                    "nano_3i1aq1cchnmbn9x5rsbap8b15akfh7wj7pwskuzi7ahz8oq6cobd99d4r3b7",
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(balance2.balance, Amount::raw(10000000));
        assert_eq!(balance2.pending, Amount::raw(0));
        assert_eq!(balance2.receivable, Amount::raw(0));
    }
}
