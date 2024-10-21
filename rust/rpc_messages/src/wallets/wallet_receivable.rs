use crate::RpcCommand;
use rsnano_core::{Amount, WalletId};
use serde::{Deserialize, Serialize};

impl RpcCommand {
    pub fn wallet_receivable(args: WalletReceivableArgs) -> Self {
        Self::WalletReceivable(args)
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct WalletReceivableArgs {
    pub wallet: WalletId,
    pub count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<Amount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_version: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_only_confirmed: Option<bool>,
}

impl WalletReceivableArgs {
    pub fn new(wallet: WalletId, count: u64) -> Self {
        Self {
            wallet,
            count,
            threshold: None,
            source: None,
            min_version: None,
            include_only_confirmed: None,
        }
    }

    pub fn builder(wallet: WalletId, count: u64) -> WalletReceivableArgsBuilder {
        WalletReceivableArgsBuilder {
            args: WalletReceivableArgs::new(wallet, count),
        }
    }
}

pub struct WalletReceivableArgsBuilder {
    args: WalletReceivableArgs,
}

impl WalletReceivableArgsBuilder {
    pub fn threshold(mut self, threshold: Amount) -> Self {
        self.args.threshold = Some(threshold);
        self
    }

    pub fn min_version(mut self) -> Self {
        self.args.min_version = Some(true);
        self
    }

    pub fn source(mut self) -> Self {
        self.args.source = Some(true);
        self
    }

    pub fn include_unconfirmed_blocks(mut self) -> Self {
        self.args.include_only_confirmed = Some(false);
        self
    }

    pub fn build(self) -> WalletReceivableArgs {
        self.args
    }
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
            to_string_pretty(&RpcCommand::wallet_receivable(WalletReceivableArgs::new(
                WalletId::zero(),
                1
            )))
            .unwrap(),
            r#"{
  "action": "wallet_receivable",
  "wallet": "0000000000000000000000000000000000000000000000000000000000000000",
  "count": 1
}"#
        )
    }

    #[test]
    fn deserialize_wallet_receivable_command_options_none() {
        let cmd = RpcCommand::wallet_receivable(WalletReceivableArgs::new(WalletId::zero(), 1));
        let serialized = serde_json::to_string_pretty(&cmd).unwrap();
        let deserialized: RpcCommand = serde_json::from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized)
    }

    #[test]
    fn serialize_wallet_receivable_command_options_some() {
        let args: WalletReceivableArgs = WalletReceivableArgs::builder(WalletId::zero(), 5)
            .threshold(Amount::raw(1000))
            .include_unconfirmed_blocks()
            .min_version()
            .source()
            .build();
        assert_eq!(
            to_string_pretty(&RpcCommand::wallet_receivable(args)).unwrap(),
            r#"{
  "action": "wallet_receivable",
  "wallet": "0000000000000000000000000000000000000000000000000000000000000000",
  "count": 5,
  "threshold": "1000",
  "source": true,
  "min_version": true,
  "include_only_confirmed": false
}"#
        )
    }

    #[test]
    fn deserialize_wallet_receivable_command_options_some() {
        let args: WalletReceivableArgs = WalletReceivableArgs::builder(WalletId::zero(), 5)
            .threshold(Amount::raw(1000))
            .include_unconfirmed_blocks()
            .min_version()
            .source()
            .build();
        let cmd = RpcCommand::wallet_receivable(args);
        let serialized = serde_json::to_string(&cmd).unwrap();
        let deserialized: RpcCommand = serde_json::from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized)
    }
}
