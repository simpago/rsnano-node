use crate::{RpcBool, RpcCommand};
use rsnano_core::{Account, WalletId};
use serde::{Deserialize, Serialize};

impl RpcCommand {
    pub fn wallet_representative_set(args: WalletRepresentativeSetArgs) -> Self {
        Self::WalletRepresentativeSet(args)
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct WalletRepresentativeSetArgs {
    pub wallet: WalletId,
    pub representative: Account,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_existing_accounts: Option<RpcBool>,
}

impl WalletRepresentativeSetArgs {
    pub fn new(wallet: WalletId, representative: Account) -> Self {
        Self {
            wallet,
            representative,
            update_existing_accounts: None,
        }
    }

    pub fn builder(wallet: WalletId, account: Account) -> WalletRepresentativeSetArgsBuilder {
        WalletRepresentativeSetArgsBuilder {
            args: WalletRepresentativeSetArgs::new(wallet, account),
        }
    }
}

pub struct WalletRepresentativeSetArgsBuilder {
    args: WalletRepresentativeSetArgs,
}

impl WalletRepresentativeSetArgsBuilder {
    pub fn update_existing_accounts(mut self) -> Self {
        self.args.update_existing_accounts = Some(true.into());
        self
    }

    pub fn build(self) -> WalletRepresentativeSetArgs {
        self.args
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct SetResponse {
    pub set: String,
}

impl SetResponse {
    pub fn new(set: bool) -> Self {
        Self {
            set: if set { "1".to_owned() } else { "0".to_owned() },
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
        let wallet_representative_set_args =
            WalletRepresentativeSetArgs::new(WalletId::zero(), Account::zero());

        let serialized = to_string(&wallet_representative_set_args).unwrap();

        let expected_json = serde_json::json!({
            "wallet": "0000000000000000000000000000000000000000000000000000000000000000",
            "representative": "nano_1111111111111111111111111111111111111111111111111111hifc8npp"
        });

        let actual_json: serde_json::Value = from_str(&serialized).unwrap();
        assert_eq!(actual_json, expected_json);
    }

    #[test]
    fn serialize_wallet_representative_set_args_update_existing_accounts_some() {
        let wallet_representative_set_args =
            WalletRepresentativeSetArgs::builder(WalletId::zero(), Account::zero())
                .update_existing_accounts()
                .build();

        let serialized = to_string(&wallet_representative_set_args).unwrap();

        let expected_json = serde_json::json!({
            "wallet": "0000000000000000000000000000000000000000000000000000000000000000",
            "representative": "nano_1111111111111111111111111111111111111111111111111111hifc8npp",
            "update_existing_accounts": "true"
        });

        let actual_json: serde_json::Value = from_str(&serialized).unwrap();
        assert_eq!(actual_json, expected_json);
    }

    #[test]
    fn deserialize_wallet_representative_set_args_update_existing_accounts_none() {
        let json_str = r#"{
            "wallet": "0000000000000000000000000000000000000000000000000000000000000000",
            "representative": "nano_1111111111111111111111111111111111111111111111111111hifc8npp"
        }"#;

        let deserialized: WalletRepresentativeSetArgs = from_str(json_str).unwrap();

        let expected = WalletRepresentativeSetArgs::new(WalletId::zero(), Account::zero());

        assert_eq!(deserialized, expected);
    }

    #[test]
    fn deserialize_wallet_representative_set_args_update_existing_accounts_some() {
        let json_str = r#"{
            "wallet": "0000000000000000000000000000000000000000000000000000000000000000",
            "representative": "nano_1111111111111111111111111111111111111111111111111111hifc8npp",
            "update_existing_accounts": "true"
        }"#;

        let deserialized: WalletRepresentativeSetArgs = from_str(json_str).unwrap();

        let expected = WalletRepresentativeSetArgs::builder(WalletId::zero(), Account::zero())
            .update_existing_accounts()
            .build();

        assert_eq!(deserialized, expected);
    }
}
