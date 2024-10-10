use crate::RpcCommand;
use rsnano_core::{Account, RawKey, WalletId};
use serde::{Deserialize, Serialize};

impl RpcCommand {
    pub fn wallet_create(seed: Option<RawKey>) -> Self {
        Self::WalletCreate(WalletCreateArgs::new(seed))
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct WalletCreateArgs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<RawKey>,
}

impl WalletCreateArgs {
    pub fn new(seed: Option<RawKey>) -> Self {
        WalletCreateArgs { seed }
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct WalletCreateDto {
    pub wallet: WalletId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_restored_account: Option<Account>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restored_count: Option<u32>,
}

impl WalletCreateDto {
    pub fn new(wallet: WalletId) -> Self {
        WalletCreateDto {
            wallet,
            last_restored_account: None,
            restored_count: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{RpcCommand, WalletCreateDto};
    use rsnano_core::{Account, RawKey, WalletId};
    use serde_json::to_string_pretty;

    #[test]
    fn serialize_wallet_create_command_seed_none() {
        assert_eq!(
            to_string_pretty(&RpcCommand::wallet_create(None)).unwrap(),
            r#"{
  "action": "wallet_create"
}"#
        )
    }

    #[test]
    fn serialize_wallet_create_command_seed_some() {
        assert_eq!(
            to_string_pretty(&RpcCommand::wallet_create(Some(RawKey::zero()))).unwrap(),
            r#"{
  "action": "wallet_create",
  "seed": "0000000000000000000000000000000000000000000000000000000000000000"
}"#
        )
    }

    #[test]
    fn deserialize_wallet_create_command_seed_none() {
        let cmd = RpcCommand::wallet_create(None);
        let serialized = serde_json::to_string_pretty(&cmd).unwrap();
        let deserialized: RpcCommand = serde_json::from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized)
    }

    #[test]
    fn deserialize_wallet_create_command_seed_some() {
        let cmd = RpcCommand::wallet_create(Some(RawKey::zero()));
        let serialized = serde_json::to_string_pretty(&cmd).unwrap();
        let deserialized: RpcCommand = serde_json::from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized)
    }

    #[test]
    fn serialize_wallet_create_dto_options_none() {
        let dto = WalletCreateDto {
            wallet: WalletId::decode_hex(
                "5D4570F8CAADE20D021490FF3E525780A380C0C7FD115F5A1EF3B4F4EF1DA03B",
            )
            .unwrap(),
            last_restored_account: None,
            restored_count: None,
        };

        let serialized = serde_json::to_string_pretty(&dto).unwrap();
        assert_eq!(
            serialized,
            r#"{
  "wallet": "5D4570F8CAADE20D021490FF3E525780A380C0C7FD115F5A1EF3B4F4EF1DA03B"
}"#
        );
    }

    #[test]
    fn serialize_wallet_create_dto_options_some() {
        let dto = WalletCreateDto {
            wallet: WalletId::decode_hex(
                "5D4570F8CAADE20D021490FF3E525780A380C0C7FD115F5A1EF3B4F4EF1DA03B",
            )
            .unwrap(),
            last_restored_account: Some(
                Account::decode_account(
                    "nano_3a1d9fj3kx3zs3worrubd6n1r69xsdapt3ykigfaq35se7agknckjxsqbxzp",
                )
                .unwrap(),
            ),
            restored_count: Some(1),
        };

        let serialized = serde_json::to_string_pretty(&dto).unwrap();
        assert_eq!(
            serialized,
            r#"{
  "wallet": "5D4570F8CAADE20D021490FF3E525780A380C0C7FD115F5A1EF3B4F4EF1DA03B",
  "last_restored_account": "nano_3a1d9fj3kx3zs3worrubd6n1r69xsdapt3ykigfaq35se7agknckjxsqbxzp",
  "restored_count": 1
}"#
        );
    }
}
