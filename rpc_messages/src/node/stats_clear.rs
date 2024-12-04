use crate::RpcCommand;

impl RpcCommand {
    pub fn stats_clear() -> Self {
        Self::StatsClear
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serialize_stats_clear() {
        let command = RpcCommand::stats_clear();
        let serialized = serde_json::to_value(command).unwrap();
        assert_eq!(serialized, json!({"action": "stats_clear"}));
    }

    #[test]
    fn deserialize_stats_clear() {
        let json = json!({"action": "stats_clear"});
        let deserialized: RpcCommand = serde_json::from_value(json).unwrap();
        assert!(matches!(deserialized, RpcCommand::StatsClear));
    }
}
