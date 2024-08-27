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
}
