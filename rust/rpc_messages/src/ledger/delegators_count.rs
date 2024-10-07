use rsnano_core::Account;
use serde::{Deserialize, Serialize};
use crate::{AccountRpcMessage, RpcCommand};

impl RpcCommand {
    pub fn delegators_count(account: Account) -> Self {
        Self::DelegatorsCount(AccountRpcMessage::new("account".to_string(), account))
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct CountDto {
    pub count: u64,
}

impl CountDto {
    pub fn new(count: u64) -> Self {
        Self { count }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serialize_delegators_count_command() {
        let command = RpcCommand::delegators_count(Account::zero());
        let serialized = serde_json::to_value(command).unwrap();
        let expected = json!({"action": "delegators_count", "account": "nano_1111111111111111111111111111111111111111111111111111hifc8npp"});
        assert_eq!(serialized, expected);
    }

    #[test]
    fn deserialize_delegators_count_command() {
        let json = r#"{"action": "delegators_count","account": "nano_1111111111111111111111111111111111111111111111111111hifc8npp"}"#;
        let deserialized: RpcCommand = serde_json::from_str(json).unwrap();
        let expected = RpcCommand::delegators_count(Account::zero());
        assert!(matches!(deserialized, expected));
    }

    #[test]
    fn serialize_count_dto() {
        let count_dto = CountDto::new(42);
        let serialized = serde_json::to_value(count_dto).unwrap();
        let expected = json!({"count": 42});
        assert_eq!(serialized, expected);
    }

    #[test]
    fn deserialize_count_dto() {
        let json = r#"{"count": 42}"#;
        let deserialized: CountDto = serde_json::from_str(json).unwrap();
        assert_eq!(deserialized, CountDto::new(42));
    }
}