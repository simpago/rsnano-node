use rsnano_core::Account;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AccountWithCountArgs {
    pub account: Account,
    pub count: u64,
}

impl AccountWithCountArgs {
    pub fn new(account: Account, count: u64) -> Self {
        Self { account, count }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_str, to_string};

    #[test]
    fn serialize_wallet_with_count_args() {
        let args = AccountWithCountArgs::new(Account::zero(), 1);

        let serialized = to_string(&args).unwrap();

        let expected_json = serde_json::json!({
            "account": "nano_1111111111111111111111111111111111111111111111111111hifc8npp",
            "count": 1
        });

        let actual_json: serde_json::Value = from_str(&serialized).unwrap();
        assert_eq!(actual_json, expected_json);
    }

    #[test]
    fn deserialize_account_with_count_args() {
        let json_str = r#"{
            "account": "nano_1111111111111111111111111111111111111111111111111111hifc8npp",
            "count": 1
        }"#;

        let deserialized: AccountWithCountArgs = from_str(json_str).unwrap();

        let expected = AccountWithCountArgs::new(Account::zero(), 1);

        assert_eq!(deserialized, expected);
    }
}
