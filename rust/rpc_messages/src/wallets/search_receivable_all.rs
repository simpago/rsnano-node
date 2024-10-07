use crate::RpcCommand;

impl RpcCommand {
    pub fn search_receivable_all() -> Self {
        Self::SearchReceivableAll
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serialize_search_receivable_all() {
        let command = RpcCommand::search_receivable_all();
        let serialized = serde_json::to_value(&command).unwrap();
        
        let expected = json!({
            "action": "search_receivable_all"
        });
        
        assert_eq!(serialized, expected);
    }

    #[test]
    fn deserialize_search_receivable_all() {
        let json_str = r#"
        {
            "action": "search_receivable_all"
        }
        "#;
        
        let deserialized: RpcCommand = serde_json::from_str(json_str).unwrap();
        
        assert!(matches!(deserialized, RpcCommand::SearchReceivableAll));
    }
}