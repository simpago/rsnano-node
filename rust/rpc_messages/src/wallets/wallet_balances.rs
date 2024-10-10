use crate::RpcCommand;
use rsnano_core::{Amount, WalletId};
use serde::{Deserialize, Serialize};

impl RpcCommand {
    pub fn wallet_balances(wallet: WalletId, threshold: Option<Amount>) -> Self {
        Self::WalletBalances(WalletBalancesArgs { wallet, threshold })
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct WalletBalancesArgs {
    pub wallet: WalletId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<Amount>,
}

#[cfg(test)]
mod tests {
    use crate::RpcCommand;
    use rsnano_core::Amount;
    use serde_json::to_string_pretty;

    #[test]
    fn serialize_wallet_balances_command_threshold_none() {
        assert_eq!(
            to_string_pretty(&RpcCommand::wallet_balances(1.into(), None)).unwrap(),
            r#"{
  "action": "wallet_balances",
  "wallet": "0000000000000000000000000000000000000000000000000000000000000001"
}"#
        )
    }

    #[test]
    fn serialize_wallet_balances_command_threshold_some() {
        assert_eq!(
            to_string_pretty(&RpcCommand::wallet_balances(1.into(), Some(Amount::zero()))).unwrap(),
            r#"{
  "action": "wallet_balances",
  "wallet": "0000000000000000000000000000000000000000000000000000000000000001",
  "threshold": "0"
}"#
        )
    }

    #[test]
    fn deserialize_wallet_balances_command_threshold_none() {
        let cmd = RpcCommand::wallet_balances(1.into(), None);
        let serialized = serde_json::to_string_pretty(&cmd).unwrap();
        let deserialized: RpcCommand = serde_json::from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized)
    }

    #[test]
    fn deserialize_wallet_balances_command_threshold_some() {
        let cmd = RpcCommand::wallet_balances(1.into(), Some(Amount::zero()));
        let serialized = serde_json::to_string_pretty(&cmd).unwrap();
        let deserialized: RpcCommand = serde_json::from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized)
    }
}
