use crate::RpcCommand;
use rsnano_core::{Account, WalletId};
use serde::{Deserialize, Serialize};

impl RpcCommand {
    pub fn account_move(wallet: WalletId, source: WalletId, accounts: Vec<Account>) -> Self {
        Self::AccountMove(AccountMoveArgs {
            wallet,
            source,
            accounts,
        })
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AccountMoveArgs {
    pub wallet: WalletId,
    pub source: WalletId,
    pub accounts: Vec<Account>,
}

#[cfg(test)]
mod tests {
    use crate::RpcCommand;
    use rsnano_core::Account;
    use serde_json::{from_str, to_string_pretty, Value};

    #[test]
    fn serialize_account_move_command() {
        let serialized = to_string_pretty(&RpcCommand::account_move(
            1.into(),
            2.into(),
            vec![Account::zero()],
        ))
        .unwrap();

        let expected = r#"{
            "action": "account_move",
            "wallet": "0000000000000000000000000000000000000000000000000000000000000001",
            "source": "0000000000000000000000000000000000000000000000000000000000000002",
            "accounts": ["nano_1111111111111111111111111111111111111111111111111111hifc8npp"]
        }"#;

        let expected_json: Value = from_str(expected).unwrap();
        let actual_json: Value = from_str(&serialized).unwrap();

        assert_eq!(expected_json, actual_json);
    }

    #[test]
    fn deserialize_account_remove_command() {
        let account = Account::from(123);
        let cmd = RpcCommand::account_move(1.into(), 2.into(), vec![account]);
        let serialized = serde_json::to_string_pretty(&cmd).unwrap();
        let deserialized: RpcCommand = serde_json::from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized)
    }
}
