#[cfg(test)]
mod tests {
    use crate::RpcCommand;
    use serde_json::json;

    #[test]
    fn serialize_unchecked_clear() {
        let command = RpcCommand::UncheckedClear;
        let serialized = serde_json::to_value(command).unwrap();
        assert_eq!(serialized, json!({"action": "unchecked_clear"}));
    }

    #[test]
    fn deserialize_unchecked_clear() {
        let json = json!({"action": "unchecked_clear"});
        let deserialized: RpcCommand = serde_json::from_value(json).unwrap();
        assert!(matches!(deserialized, RpcCommand::UncheckedClear));
    }
}
