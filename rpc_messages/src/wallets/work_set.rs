use crate::RpcCommand;
use rsnano_core::{Account, WalletId, WorkNonce};
use serde::{Deserialize, Serialize};

impl RpcCommand {
    pub fn work_set(wallet: WalletId, account: Account, work: WorkNonce) -> Self {
        Self::WorkSet(WorkSetArgs::new(wallet, account, work))
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct WorkSetArgs {
    pub wallet: WalletId,
    pub account: Account,
    pub work: WorkNonce,
}

impl WorkSetArgs {
    pub fn new(wallet: WalletId, account: Account, work: WorkNonce) -> Self {
        Self {
            wallet,
            account,
            work,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::RpcCommand;
    use rsnano_core::{Account, WalletId};
    use serde_json::to_string_pretty;

    #[test]
    fn serialize_work_set_command() {
        assert_eq!(
            to_string_pretty(&RpcCommand::work_set(
                WalletId::zero(),
                Account::zero(),
                1.into()
            ))
            .unwrap(),
            r#"{
  "action": "work_set",
  "wallet": "0000000000000000000000000000000000000000000000000000000000000000",
  "account": "nano_1111111111111111111111111111111111111111111111111111hifc8npp",
  "work": "0000000000000001"
}"#
        )
    }

    #[test]
    fn deserialize_work_set_command() {
        let cmd = RpcCommand::work_set(WalletId::zero(), Account::zero(), 1.into());
        let serialized = serde_json::to_string_pretty(&cmd).unwrap();
        let deserialized: RpcCommand = serde_json::from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized)
    }
}
