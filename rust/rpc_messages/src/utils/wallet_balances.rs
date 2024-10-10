use super::AccountBalanceDto;
use rsnano_core::Account;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct WalletBalancesDto {
    pub balances: HashMap<Account, AccountBalanceDto>,
}

impl WalletBalancesDto {
    pub fn new(balances: HashMap<Account, AccountBalanceDto>) -> Self {
        Self { balances }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::{Account, AccountBalanceDto};
    use rsnano_core::Amount;
    use serde_json;
    use std::collections::HashMap;

    #[test]
    fn serialize_wallet_balances() {
        let mut balances = HashMap::new();
        let account1: Account = 1.into();
        let account2: Account = 2.into();

        let balance1 = AccountBalanceDto::new(Amount::raw(100), Amount::raw(50), Amount::raw(50));
        let balance2 = AccountBalanceDto::new(Amount::raw(200), Amount::raw(75), Amount::raw(75));

        balances.insert(account1.clone(), balance1);
        balances.insert(account2.clone(), balance2);

        let wallet_balances = WalletBalancesDto::new(balances);

        let serialized = serde_json::to_string(&wallet_balances).unwrap();

        let deserialized: WalletBalancesDto = serde_json::from_str(&serialized).unwrap();

        assert_eq!(wallet_balances, deserialized);
    }

    #[test]
    fn deserialize_wallet_balances() {
        let json_data = r#"{
            "balances": {
                "nano_1111111111111111111111111111111111111111111111111113b8661hfk": {"balance": "100", "pending": "50", "receivable": "50"},
                "nano_11111111111111111111111111111111111111111111111111147dcwzp3c": {"balance": "200", "pending": "75", "receivable": "75"}
            }
        }"#;

        let deserialized: WalletBalancesDto = serde_json::from_str(json_data).unwrap();

        let mut balances = HashMap::new();

        let account1: Account = 1.into();
        let account2: Account = 2.into();

        let balance1 = AccountBalanceDto::new(Amount::raw(100), Amount::raw(50), Amount::raw(50));
        let balance2 = AccountBalanceDto::new(Amount::raw(200), Amount::raw(75), Amount::raw(75));

        balances.insert(account1, balance1);
        balances.insert(account2, balance2);

        let expected_wallet_balances = WalletBalancesDto::new(balances);

        assert_eq!(deserialized, expected_wallet_balances);
    }
}
