use crate::RpcCommand;

impl RpcCommand {
    pub fn available_supply() -> Self {
        Self::AvailableSupply
    }
}

#[cfg(test)]
mod tests {
    use crate::RpcCommand;
    use serde_json::{from_str, to_string_pretty};

    #[test]
    fn serialize_available_supply_command() {
        assert_eq!(
            serde_json::to_string_pretty(&RpcCommand::available_supply()).unwrap(),
            r#"{
  "action": "available_supply"
}"#
        )
    }

    #[test]
    fn derialize_account_block_count_command() {
        let cmd = RpcCommand::available_supply();
        let serialized = to_string_pretty(&cmd).unwrap();
        let deserialized: RpcCommand = from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized)
    }
}
