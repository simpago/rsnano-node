use rsnano_core::{Amount, WalletId};
use serde::{Deserialize, Serialize};

use crate::{RpcBool, RpcUsize};

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub struct WalletReceivableArgs {
    pub wallet: WalletId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<RpcUsize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<Amount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<RpcBool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_version: Option<RpcBool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_only_confirmed: Option<RpcBool>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RpcCommand;
    use rsnano_core::{Amount, WalletId};
    use serde_json::to_string_pretty;

    #[test]
    fn serialize_wallet_receivable_command_options_none() {
        assert_eq!(
            to_string_pretty(&RpcCommand::WalletReceivable(WalletReceivableArgs {
                wallet: WalletId::zero(),
                count: Some(1.into()),
                ..Default::default()
            }))
            .unwrap(),
            r#"{
  "action": "wallet_receivable",
  "wallet": "0000000000000000000000000000000000000000000000000000000000000000",
  "count": "1"
}"#
        )
    }

    #[test]
    fn deserialize_wallet_receivable_command_options_none() {
        let cmd = RpcCommand::WalletReceivable(WalletReceivableArgs {
            wallet: WalletId::zero(),
            count: Some(1.into()),
            ..Default::default()
        });
        let serialized = serde_json::to_string_pretty(&cmd).unwrap();
        let deserialized: RpcCommand = serde_json::from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized)
    }

    #[test]
    fn serialize_wallet_receivable_command_options_some() {
        let args: WalletReceivableArgs = WalletReceivableArgs {
            wallet: WalletId::zero(),
            count: Some(5.into()),
            threshold: Some(Amount::raw(1000)),
            include_only_confirmed: Some(false.into()),
            min_version: Some(true.into()),
            source: Some(true.into()),
        };
        assert_eq!(
            to_string_pretty(&RpcCommand::WalletReceivable(args)).unwrap(),
            r#"{
  "action": "wallet_receivable",
  "wallet": "0000000000000000000000000000000000000000000000000000000000000000",
  "count": "5",
  "threshold": "1000",
  "source": "true",
  "min_version": "true",
  "include_only_confirmed": "false"
}"#
        )
    }
}
