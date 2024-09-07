use rsnano_core::Amount;
use crate::{AmountRpcMessage, RpcCommand};

impl RpcCommand {
    pub fn receive_minimum_set(amount: Amount) -> Self {
        Self::ReceiveMinimumSet(AmountRpcMessage::new("amount".to_string(), amount))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsnano_core::Amount;
    use serde_json::json;

    #[test]
    fn serialize_receive_minimum_set_command() {
        let command = RpcCommand::receive_minimum_set(Amount::raw(1000000000000000000000000000000));
        let serialized = serde_json::to_value(&command).unwrap();
        
        let expected = json!({
            "action": "receive_minimum_set",
            "amount": "1000000000000000000000000000000"
        });
        
        assert_eq!(serialized, expected);
    }

    #[test]
    fn deserialize_receive_minimum_set_command() {
        let json_str = r#"
        {
            "action": "receive_minimum_set",
            "amount": "1000000000000000000000000000000"
        }
        "#;
        
        let deserialized: RpcCommand = serde_json::from_str(json_str).unwrap();
        let expected = RpcCommand::receive_minimum_set(Amount::raw(1000000000000000000000000000000));
        
        assert!(matches!(deserialized, expected));
    }
}