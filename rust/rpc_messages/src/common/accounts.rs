use rsnano_core::Account;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AccountsRpcMessage {
    pub accounts: Vec<Account>,
}

impl AccountsRpcMessage {
    pub fn new(accounts: Vec<Account>) -> Self {
        Self { accounts }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_str, to_string};

    #[test]
    fn serialize_accounts_rpc_message() {
        let dto = AccountsRpcMessage::new(vec![1.into()]);

        let serialized = to_string(&dto).unwrap();

        let expected_json = serde_json::json!({
            "accounts": ["nano_1111111111111111111111111111111111111111111111111113b8661hfk"]
        });

        let actual_json: serde_json::Value = from_str(&serialized).unwrap();
        assert_eq!(actual_json, expected_json);
    }

    #[test]
    fn deserialize_accounts_rpc_message() {
        let json_str = r#"{
            "accounts": ["nano_1111111111111111111111111111111111111111111111111113b8661hfk"]
        }"#;

        let deserialized: AccountsRpcMessage = from_str(json_str).unwrap();

        let expected = AccountsRpcMessage::new(vec![1.into()]);

        assert_eq!(deserialized, expected);
    }
}
