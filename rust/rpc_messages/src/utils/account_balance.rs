use rsnano_core::Amount;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AccountBalanceDto {
    pub balance: Amount,
    pub pending: Amount,
    pub receivable: Amount,
}

impl AccountBalanceDto {
    pub fn new(balance: Amount, pending: Amount, receivable: Amount) -> Self {
        Self {
            balance,
            pending,
            receivable,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_str, to_string};

    #[test]
    fn serialize_account_balance_dto() {
        let balance = Amount::from(1000);
        let pending = Amount::from(500);
        let receivable = Amount::from(200);

        let dto = AccountBalanceDto::new(balance, pending, receivable);

        let serialized = to_string(&dto).unwrap();

        let expected_json = serde_json::json!({
            "balance": "1000",
            "pending": "500",
            "receivable": "200"
        });

        let actual_json: serde_json::Value = from_str(&serialized).unwrap();
        assert_eq!(actual_json, expected_json);
    }

    #[test]
    fn deserialize_account_balance_dto() {
        let json_str = r#"{
            "balance": "1000",
            "pending": "500",
            "receivable": "200"
        }"#;

        let deserialized: AccountBalanceDto = from_str(json_str).unwrap();

        let balance = Amount::from(1000);
        let pending = Amount::from(500);
        let receivable = Amount::from(200);

        let expected = AccountBalanceDto::new(balance, pending, receivable);

        assert_eq!(deserialized, expected);
    }
}
