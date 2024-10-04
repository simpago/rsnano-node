use crate::RpcCommand;

impl RpcCommand {
    pub fn stop() -> Self {
        Self::Stop
    }
}

#[cfg(test)]
mod tests {
    use crate::RpcCommand;
    use serde_json::to_string_pretty;

    #[test]
    fn serialize_stop_command() {
        assert_eq!(
            to_string_pretty(&RpcCommand::Stop).unwrap(),
            r#"{
  "action": "stop"
}"#
        )
    }

    #[test]
    fn deserialize_stop_command() {
        let cmd = RpcCommand::stop();
        let serialized = serde_json::to_string_pretty(&cmd).unwrap();
        let deserialized: RpcCommand = serde_json::from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized)
    }
}
