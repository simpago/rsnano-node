use crate::{HistoryEntry, RpcCommand, RpcU64};
use rsnano_core::WalletId;
use serde::{Deserialize, Serialize};

impl RpcCommand {
    pub fn wallet_history(args: WalletHistoryArgs) -> Self {
        Self::WalletHistory(args)
    }
}

impl From<WalletId> for WalletHistoryArgs {
    fn from(value: WalletId) -> Self {
        Self::builder(value).build()
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct WalletHistoryArgs {
    pub wallet: WalletId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_since: Option<RpcU64>,
}

impl WalletHistoryArgs {
    pub fn builder(wallet: WalletId) -> WalletHistoryArgsBuilder {
        WalletHistoryArgsBuilder {
            args: Self {
                wallet,
                modified_since: None,
            },
        }
    }
}

pub struct WalletHistoryArgsBuilder {
    args: WalletHistoryArgs,
}

impl WalletHistoryArgsBuilder {
    pub fn modified_since(mut self, value: u64) -> Self {
        self.args.modified_since = Some(value.into());
        self
    }

    pub fn build(self) -> WalletHistoryArgs {
        self.args
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct WalletHistoryResponse {
    pub history: Vec<HistoryEntry>,
}

impl WalletHistoryResponse {
    pub fn new(history: Vec<HistoryEntry>) -> Self {
        Self { history }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn serialize_wallet_history() {
        let wallet_id = WalletId::decode_hex(
            "000D1BAEC8EC208142C99059B393051BAC8380F9B5A2E6B2489A277D81789F3F",
        )
        .unwrap();
        let command = RpcCommand::wallet_history(wallet_id.into());

        let expected_json = r#"{
  "action": "wallet_history",
  "wallet": "000D1BAEC8EC208142C99059B393051BAC8380F9B5A2E6B2489A277D81789F3F"
}"#;

        let serialized = serde_json::to_string_pretty(&command).unwrap();
        assert_eq!(serialized, expected_json);
    }

    #[test]
    fn serialize_wallet_history_with_modified_since() {
        let wallet_id = WalletId::decode_hex(
            "000D1BAEC8EC208142C99059B393051BAC8380F9B5A2E6B2489A277D81789F3F",
        )
        .unwrap();
        let args = WalletHistoryArgs::builder(wallet_id)
            .modified_since(1625097600)
            .build();
        let command = RpcCommand::wallet_history(args);

        let expected_json = r#"{
  "action": "wallet_history",
  "wallet": "000D1BAEC8EC208142C99059B393051BAC8380F9B5A2E6B2489A277D81789F3F",
  "modified_since": "1625097600"
}"#;

        let serialized = serde_json::to_string_pretty(&command).unwrap();
        assert_eq!(serialized, expected_json);
    }

    #[test]
    fn deserialize_wallet_history() {
        let json_data = r#"{
            "action": "wallet_history",
            "wallet": "000D1BAEC8EC208142C99059B393051BAC8380F9B5A2E6B2489A277D81789F3F"
        }"#;

        let expected_wallet_id = WalletId::decode_hex(
            "000D1BAEC8EC208142C99059B393051BAC8380F9B5A2E6B2489A277D81789F3F",
        )
        .unwrap();
        let expected_command = RpcCommand::wallet_history(expected_wallet_id.into());

        let deserialized: RpcCommand = serde_json::from_str(json_data).unwrap();
        assert_eq!(deserialized, expected_command);
    }

    #[test]
    fn deserialize_wallet_history_with_modified_since() {
        let json_data = r#"{
            "action": "wallet_history",
            "wallet": "000D1BAEC8EC208142C99059B393051BAC8380F9B5A2E6B2489A277D81789F3F",
            "modified_since": "1625097600"
        }"#;

        let expected_wallet_id = WalletId::decode_hex(
            "000D1BAEC8EC208142C99059B393051BAC8380F9B5A2E6B2489A277D81789F3F",
        )
        .unwrap();
        let args = WalletHistoryArgs::builder(expected_wallet_id)
            .modified_since(1625097600)
            .build();
        let expected_command = RpcCommand::wallet_history(args);

        let deserialized: RpcCommand = serde_json::from_str(json_data).unwrap();
        assert_eq!(deserialized, expected_command);
    }
}
