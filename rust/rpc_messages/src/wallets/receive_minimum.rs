use crate::RpcCommand;

impl RpcCommand {
    pub fn receive_minimum() -> Self {
        Self::ReceiveMinimum
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serialize_receive_minimum() {
        let command = RpcCommand::receive_minimum();
        let serialized = serde_json::to_value(&command).unwrap();
        
        let expected = json!({
            "action": "receive_minimum"
        });
        
        assert_eq!(serialized, expected);
    }

    #[test]
    fn deserialize_receive_minimum() {
        let json_str = r#"
        {
            "action": "receive_minimum"
        }
        "#;
        
        let deserialized: RpcCommand = serde_json::from_str(json_str).unwrap();
        
        assert!(matches!(deserialized, RpcCommand::ReceiveMinimum));
    }
}