use crate::RpcCommand;
use rsnano_core::{Account, WalletId};
use serde::{Deserialize, Serialize};

impl RpcCommand {
    pub fn wallet_add_watch(wallet_id: WalletId, accounts: Vec<Account>) -> Self {
        Self::WalletAddWatch(WalletAddWatchArgs {
            wallet: wallet_id,
            accounts,
        })
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct WalletAddWatchArgs {
    pub wallet: WalletId,
    pub accounts: Vec<Account>,
}

impl WalletAddWatchArgs {
    pub fn new(wallet: WalletId, accounts: Vec<Account>) -> Self {
        Self { wallet, accounts }
    }
}

#[cfg(test)]
mod tests {
    use crate::RpcCommand;
    use rsnano_core::Account;
    use serde_json::to_string_pretty;

    #[test]
    fn serialize_wallet_add_watch_command() {
        assert_eq!(
            to_string_pretty(&RpcCommand::wallet_add_watch(
                1.into(),
                vec![Account::zero()]
            ))
            .unwrap(),
            r#"{
  "action": "wallet_add_watch",
  "wallet": "0000000000000000000000000000000000000000000000000000000000000001",
  "accounts": [
    "nano_1111111111111111111111111111111111111111111111111111hifc8npp"
  ]
}"#
        )
    }

    #[test]
    fn deserialize_wallet_add_watch_command() {
        let cmd = RpcCommand::wallet_add_watch(1.into(), vec![Account::zero()]);
        let serialized = serde_json::to_string_pretty(&cmd).unwrap();
        let deserialized: RpcCommand = serde_json::from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized)
    }
}
