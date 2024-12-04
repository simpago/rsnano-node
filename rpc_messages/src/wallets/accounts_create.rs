use crate::{RpcBool, RpcCommand, RpcU64, WalletWithCountArgs};
use rsnano_core::WalletId;
use serde::{Deserialize, Serialize};

impl RpcCommand {
    pub fn accounts_create(args: AccountsCreateArgs) -> Self {
        Self::AccountsCreate(args)
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AccountsCreateArgs {
    pub wallet: WalletId,
    pub count: RpcU64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work: Option<RpcBool>,
}

impl AccountsCreateArgs {
    pub fn new(wallet: WalletId, count: u64) -> AccountsCreateArgs {
        AccountsCreateArgs {
            wallet,
            count: count.into(),
            work: None,
        }
    }

    pub fn build(wallet: WalletId, count: u64) -> AccountsCreateArgsBuilder {
        AccountsCreateArgsBuilder {
            wallet,
            count,
            work: None,
        }
    }
}

pub struct AccountsCreateArgsBuilder {
    wallet: WalletId,
    count: u64,
    work: Option<RpcBool>,
}

impl AccountsCreateArgsBuilder {
    pub fn precompute_work(mut self, precompute: bool) -> Self {
        self.work = Some(precompute.into());
        self
    }

    pub fn finish(self) -> AccountsCreateArgs {
        AccountsCreateArgs {
            wallet: self.wallet,
            count: self.count.into(),
            work: self.work,
        }
    }
}

impl From<WalletWithCountArgs> for AccountsCreateArgs {
    fn from(wallet_with_count: WalletWithCountArgs) -> Self {
        Self {
            wallet: wallet_with_count.wallet,
            count: wallet_with_count.count,
            work: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RpcCommand;
    use serde_json::to_string_pretty;

    #[test]
    fn serialize_accounts_create_command_work_some() {
        let args = AccountsCreateArgs::build(WalletId::from(1), 2)
            .precompute_work(false)
            .finish();
        assert_eq!(
            to_string_pretty(&RpcCommand::accounts_create(args)).unwrap(),
            r#"{
  "action": "accounts_create",
  "wallet": "0000000000000000000000000000000000000000000000000000000000000001",
  "count": "2",
  "work": "false"
}"#
        )
    }

    #[test]
    fn serialize_accounts_create_command_work_none() {
        let args = AccountsCreateArgs::build(WalletId::from(2), 3).finish();
        assert_eq!(
            to_string_pretty(&RpcCommand::accounts_create(args)).unwrap(),
            r#"{
  "action": "accounts_create",
  "wallet": "0000000000000000000000000000000000000000000000000000000000000002",
  "count": "3"
}"#
        )
    }

    #[test]
    fn deserialize_accounts_create_command_work_none() {
        let args = AccountsCreateArgs::build(WalletId::from(1), 2).finish();
        let cmd = RpcCommand::accounts_create(args);
        let serialized = serde_json::to_string_pretty(&cmd).unwrap();
        let deserialized: RpcCommand = serde_json::from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized)
    }

    #[test]
    fn deserialize_accounts_create_command_work_some() {
        let args = AccountsCreateArgs::build(WalletId::from(4), 5)
            .precompute_work(false)
            .finish();
        let cmd = RpcCommand::accounts_create(args);
        let serialized = serde_json::to_string(&cmd).unwrap();
        let deserialized: RpcCommand = serde_json::from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized)
    }

    #[test]
    fn test_accounts_create_builder() {
        let args = AccountsCreateArgs::build(WalletId::from(1), 5)
            .precompute_work(false)
            .finish();

        assert_eq!(args.wallet, WalletId::from(1));
        assert_eq!(args.count, 5.into());
        assert_eq!(args.work, Some(false.into()));

        let json = to_string_pretty(&args).unwrap();
        assert_eq!(
            json,
            r#"{
  "wallet": "0000000000000000000000000000000000000000000000000000000000000001",
  "count": "5",
  "work": "false"
}"#
        );
    }

    #[test]
    fn test_from_tuple_for_accounts_create_args() {
        let wallet_id = WalletId::from(2);
        let count = 3;
        let args: AccountsCreateArgs = WalletWithCountArgs::new(wallet_id, count).into();

        assert_eq!(args.wallet, wallet_id);
        assert_eq!(args.count, count.into());
        assert_eq!(args.work, None);
    }

    #[test]
    fn test_from_tuple_serialization() {
        let wallet_id = WalletId::from(2);
        let count = 3;
        let args: AccountsCreateArgs = WalletWithCountArgs::new(wallet_id, count).into();

        let json = to_string_pretty(&args).unwrap();
        assert_eq!(
            json,
            r#"{
  "wallet": "0000000000000000000000000000000000000000000000000000000000000002",
  "count": "3"
}"#
        );
    }
}
