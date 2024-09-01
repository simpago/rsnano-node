use crate::{RpcCommand, WalletRpcMessage};
use rsnano_core::WalletId;
use serde::{Deserialize, Serialize};

impl RpcCommand {
    pub fn wallet_destroy(wallet: WalletId) -> Self {
        Self::WalletDestroy(WalletRpcMessage::new(wallet))
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct WalletDestroyArgs {
    pub wallet: WalletId,
}

#[cfg(test)]
mod tests {
    use crate::RpcCommand;
    use serde_json::to_string_pretty;

    #[test]
    fn serialize_wallet_destroy_command() {
        assert_eq!(
            to_string_pretty(&RpcCommand::wallet_destroy(1.into())).unwrap(),
            r#"{
  "action": "wallet_destroy",
  "wallet": "0000000000000000000000000000000000000000000000000000000000000001"
}"#
        )
    }

    #[test]
    fn deserialize_wallet_destroy_command() {
        let cmd = RpcCommand::wallet_destroy(1.into());
        let serialized = serde_json::to_string_pretty(&cmd).unwrap();
        let deserialized: RpcCommand = serde_json::from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized)
    }
}