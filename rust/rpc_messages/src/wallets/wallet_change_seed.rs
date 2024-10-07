use rsnano_core::{Account, RawKey, WalletId};
use serde::{Deserialize, Serialize};
use crate::RpcCommand;

impl RpcCommand {
    pub fn wallet_change_seed(wallet: WalletId, seed: RawKey, count: Option<u32>) -> Self {
        RpcCommand::WalletChangeSeed(WalletChangeSeedArgs::new(wallet, seed, count))
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct WalletChangeSeedArgs {
    pub wallet: WalletId,
    pub seed: RawKey,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>
}

impl WalletChangeSeedArgs {
    pub fn new(wallet: WalletId, seed: RawKey, count: Option<u32>) -> Self {
        Self {
            wallet,
            seed,
            count,
        }
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct WalletChangeSeedDto {
    pub success: String,
    pub last_restored_account: Account,
    pub restored_count: u32,
}

impl WalletChangeSeedDto {
    pub fn new(last_restored_account: Account, restored_count: u32) -> Self {
        Self {
            success: String::new(),
            last_restored_account,
            restored_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsnano_core::{Account, RawKey, WalletId};

    #[test]
    fn serialize_wallet_change_seed_command() {
        let wallet = WalletId::zero();
        let seed = RawKey::zero();
        let count = Some(10);

        let command = RpcCommand::wallet_change_seed(wallet, seed, count);
        let serialized = serde_json::to_string(&command).unwrap();
        let deserialized: RpcCommand = serde_json::from_str(&serialized).unwrap();

        assert_eq!(command, deserialized);
    }

    #[test]
    fn serialize_wallet_change_seed_args_count_some() {
        let args = WalletChangeSeedArgs::new(
            WalletId::zero(),
            RawKey::zero(),
            Some(5),
        );

        let serialized = serde_json::to_string(&args).unwrap();
        let deserialized: WalletChangeSeedArgs = serde_json::from_str(&serialized).unwrap();

        assert_eq!(args, deserialized);
    }

    #[test]
    fn serialize_wallet_change_seed_args_count_none() {
        let args = WalletChangeSeedArgs::new(
            WalletId::zero(),
            RawKey::zero(),
            None,
        );

        let serialized = serde_json::to_string(&args).unwrap();
        assert!(!serialized.contains("count"));

        let deserialized: WalletChangeSeedArgs = serde_json::from_str(&serialized).unwrap();
        assert_eq!(args, deserialized);
    }

    #[test]
    fn serialize_wallet_change_seed_dto() {
        let dto = WalletChangeSeedDto::new(
            Account::zero(),
            15,
        );

        let serialized = serde_json::to_string(&dto).unwrap();
        let deserialized: WalletChangeSeedDto = serde_json::from_str(&serialized).unwrap();

        assert_eq!(dto, deserialized);
    }

    #[test]
    fn deserialize_wallet_change_seed_dto() {
        let json = r#"{"success":"","last_restored_account":"nano_3t6k35gi95xu6tergt6p69ck76ogmitsa8mnijtpxm9fkcm736xtoncuohr3","restored_count":15}"#;
        let deserialized: WalletChangeSeedDto = serde_json::from_str(json).unwrap();

        assert_eq!(deserialized.success, "");
        assert_eq!(deserialized.last_restored_account, Account::decode_account("nano_3t6k35gi95xu6tergt6p69ck76ogmitsa8mnijtpxm9fkcm736xtoncuohr3").unwrap());
        assert_eq!(deserialized.restored_count, 15);
    }

    #[test]
    fn deserialize_wallet_change_seed_command() {
        let json = r#"{"action":"wallet_change_seed","wallet":"000D1BAEC8EC208142C99059B393051BAC8380F9B5A2E6B2489A277D81789F3F","seed":"74F2B37AAD20F4A260F0A5B3CB3D7FB51673212263E58A380BC10474BB039CEE"}"#;
        let deserialized: RpcCommand = serde_json::from_str(json).unwrap();

        match deserialized {
            RpcCommand::WalletChangeSeed(args) => {
                assert_eq!(args.wallet, WalletId::decode_hex("000D1BAEC8EC208142C99059B393051BAC8380F9B5A2E6B2489A277D81789F3F").unwrap());
                assert_eq!(args.seed, RawKey::decode_hex("74F2B37AAD20F4A260F0A5B3CB3D7FB51673212263E58A380BC10474BB039CEE").unwrap());
                assert_eq!(args.count, None);
            },
            _ => panic!("Deserialized to wrong variant"),
        }
    }

    #[test]
    fn deserialize_wallet_change_seed_args_count_none() {
        let json = r#"{"wallet":"000D1BAEC8EC208142C99059B393051BAC8380F9B5A2E6B2489A277D81789F3F","seed":"74F2B37AAD20F4A260F0A5B3CB3D7FB51673212263E58A380BC10474BB039CEE"}"#;
        let deserialized: WalletChangeSeedArgs = serde_json::from_str(json).unwrap();

        assert_eq!(deserialized.wallet, WalletId::decode_hex("000D1BAEC8EC208142C99059B393051BAC8380F9B5A2E6B2489A277D81789F3F").unwrap());
        assert_eq!(deserialized.seed, RawKey::decode_hex("74F2B37AAD20F4A260F0A5B3CB3D7FB51673212263E58A380BC10474BB039CEE").unwrap());
        assert_eq!(deserialized.count, None);
    }

    #[test]
    fn deserialize_wallet_change_seed_args_count_some() {
        let json = r#"{"wallet":"000D1BAEC8EC208142C99059B393051BAC8380F9B5A2E6B2489A277D81789F3F","seed":"74F2B37AAD20F4A260F0A5B3CB3D7FB51673212263E58A380BC10474BB039CEE","count":5}"#;
        let deserialized: WalletChangeSeedArgs = serde_json::from_str(json).unwrap();

        assert_eq!(deserialized.wallet, WalletId::decode_hex("000D1BAEC8EC208142C99059B393051BAC8380F9B5A2E6B2489A277D81789F3F").unwrap());
        assert_eq!(deserialized.seed, RawKey::decode_hex("74F2B37AAD20F4A260F0A5B3CB3D7FB51673212263E58A380BC10474BB039CEE").unwrap());
        assert_eq!(deserialized.count, Some(5));
    }
}


