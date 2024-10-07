use crate::RpcCommand;
use rsnano_core::Account;
use rsnano_core::{Amount, BlockHash};
use serde::{Deserialize, Serialize};

impl RpcCommand {
    pub fn account_info(
        account: Account,
        representative: Option<bool>,
        weight: Option<bool>,
        pending: Option<bool>,
        receivable: Option<bool>,
        include_confirmed: Option<bool>,
    ) -> Self {
        Self::AccountInfo(AccountInfoArgs {
            account,
            representative,
            weight,
            pending,
            receivable,
            include_confirmed,
        })
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AccountInfoArgs {
    pub account: Account,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub representative: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receivable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_confirmed: Option<bool>,
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AccountInfoDto {
    pub frontier: BlockHash,
    pub open_block: BlockHash,
    pub representative_block: BlockHash,
    pub balance: Amount,
    pub modified_timestamp: u64,
    pub block_count: u64,
    pub account_version: u8,
    pub confirmed_height: Option<u64>,
    pub confirmation_height_frontier: Option<BlockHash>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub representative: Option<Account>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<Amount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending: Option<Amount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receivable: Option<Amount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmed_balance: Option<Amount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmed_pending: Option<Amount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmed_receivable: Option<Amount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmed_representative: Option<Account>,
}

impl AccountInfoDto {
    pub fn new(
        frontier: BlockHash,
        open_block: BlockHash,
        representative_block: BlockHash,
        balance: Amount,
        modified_timestamp: u64,
        block_count: u64,
        account_version: u8,
    ) -> Self {
        Self {
            frontier,
            open_block,
            representative_block,
            balance,
            modified_timestamp,
            block_count,
            account_version,
            confirmed_height: None,
            confirmation_height_frontier: None,
            // Initialize new fields with None
            representative: None,
            weight: None,
            pending: None,
            receivable: None,
            confirmed_balance: None,
            confirmed_pending: None,
            confirmed_receivable: None,
            confirmed_representative: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_str, to_string_pretty};

    #[test]
    fn serialize_account_info_command() {
        assert_eq!(
            serde_json::to_string_pretty(&RpcCommand::account_info(
                Account::from(123),
                None,
                None,
                None,
                None,
                None
            ))
            .unwrap(),
            r#"{
  "action": "account_info",
  "account": "nano_111111111111111111111111111111111111111111111111115uwdgas549"
}"#
        )
    }

    #[test]
    fn derialize_account_info_command() {
        let account = Account::from(123);
        let cmd = RpcCommand::account_info(account, None, None, None, None, None);
        let serialized = to_string_pretty(&cmd).unwrap();
        let deserialized: RpcCommand = from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized)
    }

    #[test]
    fn serialize_account_info_dto_with_none_values() {
        let account_info = AccountInfoDto {
            frontier: BlockHash::zero(),
            open_block: BlockHash::zero(),
            representative_block: BlockHash::zero(),
            balance: Amount::raw(1000),
            modified_timestamp: 1234567890,
            block_count: 100,
            account_version: 1,
            confirmed_height: Some(99),
            confirmation_height_frontier: Some(BlockHash::zero()),
            representative: Some(Account::zero()),
            weight: Some(Amount::raw(2000)),
            pending: Some(Amount::raw(300)),
            receivable: Some(Amount::raw(400)),
            confirmed_balance: None,
            confirmed_pending: None,
            confirmed_receivable: None,
            confirmed_representative: None,
        };

        let serialized = to_string_pretty(&account_info).unwrap();
        println!("Serialized AccountInfoDto:\n{}", serialized);

        let deserialized: AccountInfoDto = from_str(&serialized).unwrap();
        assert_eq!(account_info, deserialized);
    }

    #[test]
    fn deserialize_account_info_dto_with_none_values() {
        let account_info = AccountInfoDto::new(
            BlockHash::zero(),
            BlockHash::zero(),
            BlockHash::zero(),
            Amount::raw(1000),
            1234567890,
            100,
            1,
        );

        let serialized = to_string_pretty(&account_info).unwrap();
        println!("Serialized AccountInfoDto with None values:\n{}", serialized);

        let deserialized: AccountInfoDto = from_str(&serialized).unwrap();
        assert_eq!(account_info, deserialized);

        // Check that None fields are not serialized
        //assert!(!serialized.contains("representative"));
        assert!(!serialized.contains("weight"));
        assert!(!serialized.contains("pending"));
        assert!(!serialized.contains("receivable"));
        assert!(!serialized.contains("confirmed_balance"));
        assert!(!serialized.contains("confirmed_pending"));
        assert!(!serialized.contains("confirmed_receivable"));
        assert!(!serialized.contains("confirmed_representative"));
    }

    fn create_account_info_dto_with_some_values() -> AccountInfoDto {
        AccountInfoDto {
            frontier: BlockHash::zero(),
            open_block: BlockHash::zero(),
            representative_block: BlockHash::zero(),
            balance: Amount::from(1000),
            modified_timestamp: 1234567890,
            block_count: 100,
            account_version: 1,
            confirmed_height: Some(99),
            confirmation_height_frontier: Some(BlockHash::zero()),
            representative: Some(Account::zero()),
            weight: Some(Amount::from(2000)),
            pending: Some(Amount::from(300)),
            receivable: Some(Amount::from(400)),
            confirmed_balance: Some(Amount::from(950)),
            confirmed_pending: Some(Amount::from(250)),
            confirmed_receivable: Some(Amount::from(350)),
            confirmed_representative: Some(Account::zero()),
        }
    }

    #[test]
    fn serialize_account_info_dto_with_some_values() {
        let account_info = create_account_info_dto_with_some_values();
        let serialized = to_string_pretty(&account_info).unwrap();
        println!("Serialized AccountInfoDto with Some values:\n{}", serialized);

        // Check that all fields are present in the serialized output
        assert!(serialized.contains("frontier"));
        assert!(serialized.contains("representative"));
        assert!(serialized.contains("weight"));
        assert!(serialized.contains("pending"));
        assert!(serialized.contains("receivable"));
        assert!(serialized.contains("confirmed_balance"));
        assert!(serialized.contains("confirmed_pending"));
        assert!(serialized.contains("confirmed_receivable"));
        assert!(serialized.contains("confirmed_representative"));
    }

    #[test]
    fn deserialize_account_info_dto_with_some_values() {
        let account_info = create_account_info_dto_with_some_values();
        let serialized = to_string_pretty(&account_info).unwrap();
        let deserialized: AccountInfoDto = from_str(&serialized).unwrap();
        assert_eq!(account_info, deserialized);
    }

    #[test]
    fn serialize_account_info_args() {
        let args = AccountInfoArgs {
            account: Account::zero(),
            representative: Some(true),
            weight: Some(false),
            pending: None,
            receivable: Some(true),
            include_confirmed: None,
        };

        let serialized = to_string_pretty(&args).unwrap();
        println!("Serialized AccountInfoArgs:\n{}", serialized);

        // Check that fields with Some values are serialized
        assert!(serialized.contains("account"));
        assert!(serialized.contains("representative"));
        assert!(serialized.contains("weight"));
        assert!(serialized.contains("receivable"));

        // Check that fields with None values are not serialized
        assert!(!serialized.contains("pending"));
        assert!(!serialized.contains("include_confirmed"));

        let deserialized: AccountInfoArgs = from_str(&serialized).unwrap();
        assert_eq!(args, deserialized);
    }

    #[test]
    fn serialize_account_info_command_with_some_args() {
        let command = RpcCommand::account_info(
            Account::zero(),
            Some(true),
            Some(false),
            None,
            Some(true),
            None,
        );

        let serialized = to_string_pretty(&command).unwrap();
        println!("Serialized AccountInfo command:\n{}", serialized);

        // Check that fields with Some values are serialized
        assert!(serialized.contains("account"));
        assert!(serialized.contains("representative"));
        assert!(serialized.contains("weight"));
        assert!(serialized.contains("receivable"));

        // Check that fields with None values are not serialized
        assert!(!serialized.contains("pending"));
        assert!(!serialized.contains("include_confirmed"));

        let deserialized: RpcCommand = from_str(&serialized).unwrap();
    }
}
