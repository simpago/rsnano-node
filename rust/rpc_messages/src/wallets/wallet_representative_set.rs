use serde::{Deserialize, Serialize};
use crate::RpcCommand;
use super::WalletWithAccountArgs;

impl RpcCommand {
    pub fn wallet_representative_set(wallet_with_account: WalletWithAccountArgs, update_existing_accounts: Option<bool>) -> Self {
        Self::WalletRepresentativeSet(WalletRepresentativeSetArgs::new(wallet_with_account, update_existing_accounts))
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]

 pub struct WalletRepresentativeSetArgs {
    #[serde(flatten)]
    pub wallet_with_account: WalletWithAccountArgs,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_existing_accounts: Option<bool>,
}

impl WalletRepresentativeSetArgs {
    pub fn new(wallet_with_account: WalletWithAccountArgs, update_existing_accounts: Option<bool>) -> Self {
        Self {
            wallet_with_account,
            update_existing_accounts,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsnano_core::{Account, WalletId};
    use serde_json::{from_str, to_string};

    #[test]
    fn serialize_wallet_representative_set_args_update_existing_accounts_none() {
        let wallet_with_account = WalletWithAccountArgs::new(WalletId::zero(), Account::zero());
        let wallet_representative_set_args = WalletRepresentativeSetArgs::new(wallet_with_account, None);

        let serialized = to_string(&wallet_representative_set_args).unwrap();

        let expected_json = serde_json::json!({
            "wallet": "0000000000000000000000000000000000000000000000000000000000000000",
            "account": "nano_1111111111111111111111111111111111111111111111111111hifc8npp"
        });

        let actual_json: serde_json::Value = from_str(&serialized).unwrap();
        assert_eq!(actual_json, expected_json);
    }

    #[test]
    fn serialize_wallet_representative_set_args_update_existing_accounts_some() {
        let wallet_with_account = WalletWithAccountArgs::new(WalletId::zero(), Account::zero());
        let wallet_representative_set_args = WalletRepresentativeSetArgs::new(wallet_with_account, Some(true));

        let serialized = to_string(&wallet_representative_set_args).unwrap();

        let expected_json = serde_json::json!({
            "wallet": "0000000000000000000000000000000000000000000000000000000000000000",
            "account": "nano_1111111111111111111111111111111111111111111111111111hifc8npp",
            "update_existing_accounts": true
        });

        let actual_json: serde_json::Value = from_str(&serialized).unwrap();
        assert_eq!(actual_json, expected_json);
    }

    #[test]
    fn deserialize_wallet_representative_set_args_update_existing_accounts_none() {
        let json_str = r#"{
            "wallet": "0000000000000000000000000000000000000000000000000000000000000000",
            "account": "nano_1111111111111111111111111111111111111111111111111111hifc8npp"
        }"#;

        let deserialized: WalletRepresentativeSetArgs = from_str(json_str).unwrap();

        let wallet_with_account = WalletWithAccountArgs::new(WalletId::zero(), Account::zero());
        let expected = WalletRepresentativeSetArgs::new(wallet_with_account, None);

        assert_eq!(deserialized, expected);
    }

    #[test]
    fn deserialize_wallet_representative_set_args_update_existing_accounts_some() {
        let json_str = r#"{
            "wallet": "0000000000000000000000000000000000000000000000000000000000000000",
            "account": "nano_1111111111111111111111111111111111111111111111111111hifc8npp",
            "update_existing_accounts": true
        }"#;

        let deserialized: WalletRepresentativeSetArgs = from_str(json_str).unwrap();

        let wallet_with_account = WalletWithAccountArgs::new(WalletId::zero(), Account::zero());
        let expected = WalletRepresentativeSetArgs::new(wallet_with_account, Some(true));

        assert_eq!(deserialized, expected);
    }
}
