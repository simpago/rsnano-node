use crate::RpcCommand;

impl RpcCommand {
    pub fn frontier_count() -> Self {
        Self::FrontierCount
    }
}

#[cfg(test)]
mod tests {
    use crate::RpcCommand;
    use serde_json::to_string_pretty;

    #[test]
    fn serialize_frontier_count_command() {
        assert_eq!(
            to_string_pretty(&RpcCommand::FrontierCount).unwrap(),
            r#"{
  "action": "frontier_count"
}"#
        )
    }

    #[test]
    fn deserialize_frontier_count_command() {
        let json_str = r#"{
    "action": "frontier_count"
    }"#;
        let deserialized: RpcCommand = serde_json::from_str(json_str).unwrap();
        let expected_command = RpcCommand::frontier_count();
        assert_eq!(deserialized, expected_command);
    }
}
