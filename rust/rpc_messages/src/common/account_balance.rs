use rsnano_core::Amount;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AccountBalanceResponse {
    pub balance: Amount,
    pub pending: Amount,
    pub receivable: Amount,
}

#[cfg(test)]
mod tests {
    use crate::common::AccountBalanceResponse;
    use rsnano_core::Amount;

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