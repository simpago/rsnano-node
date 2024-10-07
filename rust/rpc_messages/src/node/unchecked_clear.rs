use crate::RpcCommand;

impl RpcCommand {
    pub fn unchecked_clear() -> Self {
        Self::UncheckedClear
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use super::*;

    #[test]
    fn serialize_unchecked_clear() {
        let command = RpcCommand::unchecked_clear();
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