use rsnano_core::{Amount, WalletId};
use serde::{Deserialize, Serialize};
use crate::RpcCommand;

use super::{wallet_with_count, WalletWithCountArgs};

impl RpcCommand {
    pub fn wallet_receivable(
        wallet_with_count: WalletWithCountArgs,
        threshold: Option<Amount>,
        source: Option<bool>,
        min_version: Option<bool>,
        include_only_confirmed: Option<bool>
    ) -> Self {
        Self::WalletReceivable(WalletReceivableArgs {
            wallet_with_count,
            threshold,
            source,
            min_version,
            include_only_confirmed
        })
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct WalletReceivableArgs {
    #[serde(flatten)]
    pub wallet_with_count: WalletWithCountArgs,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<Amount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_version: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_only_confirmed: Option<bool>
}

impl WalletReceivableArgs {
    pub fn new(
        wallet: WalletId,
        count: u64,
        threshold: Option<Amount>,
        source: Option<bool>,
        min_version: Option<bool>,
        include_only_confirmed: Option<bool>
    ) -> Self {
        Self {
            wallet_with_count: WalletWithCountArgs { wallet, count },
            threshold,
            source,
            min_version,
            include_only_confirmed
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RpcCommand;
    use rsnano_core::{WalletId, Amount};
    use serde_json::to_string_pretty;

    #[test]
    fn serialize_wallet_receivable_command_options_none() {
        assert_eq!(
            to_string_pretty(&RpcCommand::wallet_receivable(
                WalletWithCountArgs { wallet: WalletId::zero(), count: 1 },
                None, None, None, None
            )).unwrap(),
            r#"{
  "action": "wallet_receivable",
  "wallet": "0000000000000000000000000000000000000000000000000000000000000000",
  "count": 1
}"#
        )
    }

    #[test]
    fn deserialize_wallet_receivable_command_options_none() {
        let cmd = RpcCommand::wallet_receivable(
            WalletWithCountArgs { wallet: WalletId::zero(), count: 1 },
            None, None, None, None
        );
        let serialized = serde_json::to_string_pretty(&cmd).unwrap();
        let deserialized: RpcCommand = serde_json::from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized)
    }

    #[test]
    fn serialize_wallet_receivable_command_options_some() {
        assert_eq!(
            to_string_pretty(&RpcCommand::wallet_receivable(
                WalletWithCountArgs { wallet: WalletId::zero(), count: 5 },
                Some(Amount::raw(1000)),
                Some(true),
                Some(false),
                Some(true)
            )).unwrap(),
            r#"{
  "action": "wallet_receivable",
  "wallet": "0000000000000000000000000000000000000000000000000000000000000000",
  "count": 5,
  "threshold": "1000",
  "source": true,
  "min_version": false,
  "include_only_confirmed": true
}"#
        )
    }

    #[test]
    fn deserialize_wallet_receivable_command_options_some() {
        let cmd = RpcCommand::wallet_receivable(
            WalletWithCountArgs { wallet: WalletId::zero(), count: 5 },
            Some(Amount::raw(1000)),
            Some(true),
            Some(false),
            Some(true)
        );
        let serialized = serde_json::to_string(&cmd).unwrap();
        let deserialized: RpcCommand = serde_json::from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized)
    }
}