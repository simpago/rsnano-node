use crate::{RpcBool, RpcCommand, RpcU32};
use rsnano_core::WalletId;
use serde::{Deserialize, Serialize};

impl RpcCommand {
    pub fn account_create(args: AccountCreateArgs) -> Self {
        Self::AccountCreate(args)
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AccountCreateArgs {
    pub wallet: WalletId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<RpcU32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work: Option<RpcBool>,
}

impl AccountCreateArgs {
    pub fn new(wallet: WalletId) -> AccountCreateArgs {
        AccountCreateArgs {
            wallet,
            index: None,
            work: None,
        }
    }

    pub fn builder(wallet: WalletId) -> AccountCreateArgsBuilder {
        AccountCreateArgsBuilder {
            wallet,
            index: None,
            work: None,
        }
    }
}

pub struct AccountCreateArgsBuilder {
    wallet: WalletId,
    index: Option<RpcU32>,
    work: Option<RpcBool>,
}

impl AccountCreateArgsBuilder {
    pub fn with_index(mut self, index: u32) -> Self {
        self.index = Some(index.into());
        self
    }

    pub fn without_precomputed_work(mut self) -> Self {
        self.work = Some(false.into());
        self
    }

    pub fn build(self) -> AccountCreateArgs {
        AccountCreateArgs {
            wallet: self.wallet,
            index: self.index,
            work: self.work,
        }
    }
}

impl From<WalletId> for AccountCreateArgs {
    fn from(wallet: WalletId) -> Self {
        Self {
            wallet,
            index: None,
            work: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::to_string_pretty;

    #[test]
    fn serialize_account_create_command_options_none() {
        assert_eq!(
            to_string_pretty(&RpcCommand::AccountCreate(WalletId::zero().into())).unwrap(),
            r#"{
  "action": "account_create",
  "wallet": "0000000000000000000000000000000000000000000000000000000000000000"
}"#
        )
    }

    #[test]
    fn serialize_account_create_command_options_some() {
        let args = AccountCreateArgs::builder(WalletId::from(1))
            .with_index(1)
            .without_precomputed_work()
            .build();
        assert_eq!(
            to_string_pretty(&RpcCommand::AccountCreate(args)).unwrap(),
            r#"{
  "action": "account_create",
  "wallet": "0000000000000000000000000000000000000000000000000000000000000001",
  "index": "1",
  "work": "false"
}"#
        )
    }

    #[test]
    fn deserialize_account_create_command_options_none() {
        let cmd = RpcCommand::AccountCreate(WalletId::from(1).into());
        let serialized = serde_json::to_string_pretty(&cmd).unwrap();
        let deserialized: RpcCommand = serde_json::from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized)
    }

    #[test]
    fn deserialize_account_create_command_options_some() {
        let args = AccountCreateArgs::builder(WalletId::from(1))
            .with_index(1)
            .without_precomputed_work()
            .build();
        let cmd = RpcCommand::AccountCreate(args);
        let serialized = serde_json::to_string_pretty(&cmd).unwrap();
        let deserialized: RpcCommand = serde_json::from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized)
    }

    #[test]
    fn test_account_create_builder() {
        let args = AccountCreateArgs::builder(WalletId::from(1))
            .with_index(2)
            .without_precomputed_work()
            .build();

        assert_eq!(args.wallet, WalletId::from(1));
        assert_eq!(args.index, Some(2.into()));
        assert_eq!(args.work, Some(false.into()));

        let json = to_string_pretty(&args).unwrap();
        assert_eq!(
            json,
            r#"{
  "wallet": "0000000000000000000000000000000000000000000000000000000000000001",
  "index": "2",
  "work": "false"
}"#
        );
    }

    #[test]
    fn test_from_wallet_id_for_account_create_args() {
        let wallet_id = WalletId::from(1);
        let args: AccountCreateArgs = wallet_id.into();

        assert_eq!(args.wallet, wallet_id);
        assert_eq!(args.index, None);
        assert_eq!(args.work, None);
    }

    #[test]
    fn test_from_wallet_id_serialization() {
        let wallet_id = WalletId::from(1);
        let args: AccountCreateArgs = wallet_id.into();

        let json = to_string_pretty(&args).unwrap();
        assert_eq!(
            json,
            r#"{
  "wallet": "0000000000000000000000000000000000000000000000000000000000000001"
}"#
        );
    }
}
