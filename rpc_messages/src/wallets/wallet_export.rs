use crate::{common::WalletRpcMessage, RpcCommand};
use rsnano_core::WalletId;
use serde::{Deserialize, Serialize};

impl RpcCommand {
    pub fn wallet_export(wallet: WalletId) -> Self {
        Self::WalletExport(WalletRpcMessage::new(wallet))
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct JsonResponse {
    pub json: String,
}

impl JsonResponse {
    pub fn new(json: impl Into<String>) -> Self {
        Self { json: json.into() }
    }
}

#[cfg(test)]
mod tests {
    use crate::wallets::JsonResponse;

    use super::RpcCommand;
    use rsnano_core::WalletId;
    use serde_json::to_string_pretty;

    #[test]
    fn serialize_wallet_export_command() {
        assert_eq!(
            to_string_pretty(&RpcCommand::wallet_export(WalletId::zero())).unwrap(),
            r#"{
  "action": "wallet_export",
  "wallet": "0000000000000000000000000000000000000000000000000000000000000000"
}"#
        )
    }

    #[test]
    fn deserialize_wallet_export_command() {
        let cmd = RpcCommand::wallet_export(WalletId::zero());
        let serialized = serde_json::to_string_pretty(&cmd).unwrap();
        let deserialized: RpcCommand = serde_json::from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized)
    }

    #[test]
    fn serialize_json_dto() {
        let json_dto = JsonResponse::new("foobar");
        let serialized = serde_json::to_string(&json_dto).unwrap();

        let expected_serialized = r#"{"json":"foobar"}"#;

        assert_eq!(serialized, expected_serialized);
    }

    #[test]
    fn deserialize_json_dto() {
        let json_str = r#"{"json":"foobar"}"#;
        let deserialized: JsonResponse = serde_json::from_str(json_str).unwrap();
        let expected = JsonResponse::new("foobar");
        assert_eq!(deserialized, expected);
    }
}
