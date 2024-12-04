use crate::RpcCommand;

impl RpcCommand {
    pub fn key_create() -> Self {
        Self::KeyCreate
    }
}

#[cfg(test)]
mod tests {
    use crate::RpcCommand;
    use serde_json::to_string_pretty;

    #[test]
    fn serialize_key_create_command() {
        assert_eq!(
            to_string_pretty(&RpcCommand::KeyCreate).unwrap(),
            r#"{
  "action": "key_create"
}"#
        )
    }

    #[test]
    fn deserialize_key_create_command() {
        let cmd = RpcCommand::key_create();
        let serialized = serde_json::to_string_pretty(&cmd).unwrap();
        let deserialized: RpcCommand = serde_json::from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized)
    }
}
