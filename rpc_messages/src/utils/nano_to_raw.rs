use crate::{common::AmountRpcMessage, RpcCommand};
use rsnano_core::Amount;

impl RpcCommand {
    pub fn nano_to_raw(amount: u64) -> Self {
        Self::NanoToRaw(AmountRpcMessage::new(Amount::raw(amount as u128)))
    }
}

#[cfg(test)]
mod tests {
    use crate::RpcCommand;
    use serde_json::{from_str, to_string_pretty};

    #[test]
    fn serialize_nano_to_raw_command() {
        assert_eq!(
            to_string_pretty(&RpcCommand::nano_to_raw(1)).unwrap(),
            r#"{
  "action": "nano_to_raw",
  "amount": "1"
}"#
        );
    }

    #[test]
    fn deserialize_nano_to_raw_command() {
        let cmd = RpcCommand::nano_to_raw(1);
        let serialized = to_string_pretty(&cmd).unwrap();
        let deserialized: RpcCommand = from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized);
    }
}
