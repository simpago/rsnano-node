mod account_info;

use super::RpcCommand;
pub use account_info::*;
use rsnano_core::Account;

impl RpcCommand {
    pub fn account_info(account: Account) -> Self {
        Self::AccountInfo(AccountInfoArgs { account })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsnano_core::Account;

    #[test]
    fn deserialize() {
        let account = Account::from(123);
        let cmd = RpcCommand::account_info(account);
        let serialized = serde_json::to_string_pretty(&cmd).unwrap();
        let deserialized: RpcCommand = serde_json::from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized)
    }
}
