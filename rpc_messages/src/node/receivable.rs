use indexmap::IndexMap;
use rsnano_core::{Account, Amount, BlockHash};
use serde::{Deserialize, Serialize};

use crate::{RpcBool, RpcU64, RpcU8};

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub struct ReceivableArgs {
    pub account: Account,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<RpcU64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<RpcU64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<Amount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<RpcBool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_version: Option<RpcBool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sorting: Option<RpcBool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_only_confirmed: Option<RpcBool>,
}

impl ReceivableArgs {
    pub fn new(account: Account) -> Self {
        Self {
            account,
            ..Default::default()
        }
    }

    pub fn build(account: impl Into<Account>) -> ReceivableArgsBuilder {
        ReceivableArgsBuilder {
            args: ReceivableArgs::new(account.into()),
        }
    }
}

impl From<Account> for ReceivableArgs {
    fn from(value: Account) -> Self {
        Self {
            account: value,
            ..Default::default()
        }
    }
}

pub struct ReceivableArgsBuilder {
    args: ReceivableArgs,
}

impl ReceivableArgsBuilder {
    pub fn threshold(mut self, threshold: Amount) -> Self {
        self.args.threshold = Some(threshold);
        self
    }

    pub fn include_only_confirmed(mut self, include: bool) -> Self {
        self.args.include_only_confirmed = Some(include.into());
        self
    }

    pub fn min_version(mut self) -> Self {
        self.args.min_version = Some(true.into());
        self
    }

    pub fn sort(mut self) -> Self {
        self.args.sorting = Some(true.into());
        self
    }

    pub fn source(mut self) -> Self {
        self.args.source = Some(true.into());
        self
    }

    pub fn count(mut self, count: u64) -> Self {
        self.args.count = Some(count.into());
        self
    }

    pub fn finish(self) -> ReceivableArgs {
        self.args
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ReceivableResponse {
    Simple(ReceivableSimple),
    Source(ReceivableSource),
    Threshold(ReceivableThreshold),
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ReceivableSimple {
    pub blocks: Vec<BlockHash>,
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ReceivableThreshold {
    pub blocks: IndexMap<BlockHash, Amount>,
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ReceivableSource {
    pub blocks: IndexMap<BlockHash, SourceInfo>,
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AccountsReceivableResponse {
    Simple(AccountsReceivableSimple),
    Source(AccountsReceivableSource),
    Threshold(AccountsReceivableThreshold),
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AccountsReceivableSimple {
    pub blocks: IndexMap<Account, Vec<BlockHash>>,
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AccountsReceivableThreshold {
    pub blocks: IndexMap<Account, IndexMap<BlockHash, Amount>>,
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AccountsReceivableSource {
    pub blocks: IndexMap<Account, IndexMap<BlockHash, SourceInfo>>,
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct SourceInfo {
    pub amount: Amount,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Account>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_version: Option<RpcU8>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RpcCommand;
    use serde_json::to_string_pretty;

    #[test]
    fn serialize_receivable_command() {
        assert_eq!(
            to_string_pretty(&RpcCommand::Receivable(ReceivableArgs {
                account: Account::zero(),
                count: Some(1.into()),
                ..Default::default()
            }))
            .unwrap(),
            r#"{
  "action": "receivable",
  "account": "nano_1111111111111111111111111111111111111111111111111111hifc8npp",
  "count": "1"
}"#
        )
    }

    #[test]
    fn deserialize_receivable_command() {
        let cmd = RpcCommand::Receivable(ReceivableArgs {
            account: Account::zero(),
            count: Some(1.into()),
            ..Default::default()
        });
        let serialized = serde_json::to_string_pretty(&cmd).unwrap();
        let deserialized: RpcCommand = serde_json::from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized)
    }

    #[test]
    fn serialize_wallet_receivable_dto_blocks() {
        let mut blocks = IndexMap::new();
        blocks.insert(Account::zero(), vec![BlockHash::zero()]);
        let works = AccountsReceivableResponse::Simple(AccountsReceivableSimple { blocks });
        let expected_json = r#"{"blocks":{"nano_1111111111111111111111111111111111111111111111111111hifc8npp":["0000000000000000000000000000000000000000000000000000000000000000"]}}"#;

        let serialized = serde_json::to_string(&works).unwrap();

        assert_eq!(serialized, expected_json);
    }

    #[test]
    fn deserialize_wallet_receivable_dto_blocks() {
        let json_data = r#"{"blocks":{"nano_1111111111111111111111111111111111111111111111111111hifc8npp":["0000000000000000000000000000000000000000000000000000000000000000"]}}"#;
        let works: AccountsReceivableResponse = serde_json::from_str(json_data).unwrap();

        let mut expected_blocks = IndexMap::new();
        expected_blocks.insert(Account::zero(), vec![BlockHash::zero()]);

        let expected_works = AccountsReceivableResponse::Simple(AccountsReceivableSimple {
            blocks: expected_blocks,
        });

        assert_eq!(works, expected_works);
    }

    #[test]
    fn serialize_wallet_receivable_dto_threshold() {
        let mut blocks = IndexMap::new();
        let mut inner_map = IndexMap::new();
        inner_map.insert(BlockHash::zero(), Amount::from(1000));
        blocks.insert(Account::zero(), inner_map);

        let works = AccountsReceivableResponse::Threshold(AccountsReceivableThreshold { blocks });

        let expected_json = r#"{"blocks":{"nano_1111111111111111111111111111111111111111111111111111hifc8npp":{"0000000000000000000000000000000000000000000000000000000000000000":"1000"}}}"#;
        let serialized = serde_json::to_string(&works).unwrap();

        assert_eq!(serialized, expected_json);
    }

    #[test]
    fn deserialize_wallet_receivable_dto_threshold() {
        let json_data = r#"{"blocks":{"nano_1111111111111111111111111111111111111111111111111111hifc8npp":{"0000000000000000000000000000000000000000000000000000000000000000":"1000"}}}"#;
        let works: AccountsReceivableResponse = serde_json::from_str(json_data).unwrap();

        let mut expected_blocks = IndexMap::new();
        let mut inner_map = IndexMap::new();
        inner_map.insert(BlockHash::zero(), Amount::from(1000));
        expected_blocks.insert(Account::zero(), inner_map);

        let expected_works = AccountsReceivableResponse::Threshold(AccountsReceivableThreshold {
            blocks: expected_blocks,
        });

        assert_eq!(works, expected_works);
    }

    #[test]
    fn serialize_wallet_receivable_dto_source() {
        let mut blocks = IndexMap::new();
        let mut inner_map = IndexMap::new();
        inner_map.insert(
            BlockHash::zero(),
            SourceInfo {
                amount: Amount::from(1000),
                source: Some(Account::zero()),
                min_version: None,
            },
        );
        blocks.insert(Account::zero(), inner_map);

        let works = AccountsReceivableResponse::Source(AccountsReceivableSource { blocks });

        let expected_json = r#"{"blocks":{"nano_1111111111111111111111111111111111111111111111111111hifc8npp":{"0000000000000000000000000000000000000000000000000000000000000000":{"amount":"1000","source":"nano_1111111111111111111111111111111111111111111111111111hifc8npp"}}}}"#;
        let serialized = serde_json::to_string(&works).unwrap();

        assert_eq!(serialized, expected_json);
    }

    #[test]
    fn deserialize_wallet_receivable_dto_source() {
        let json_data = r#"{"blocks":{"nano_1111111111111111111111111111111111111111111111111111hifc8npp":{"0000000000000000000000000000000000000000000000000000000000000000":{"amount":"1000","source":"nano_1111111111111111111111111111111111111111111111111111hifc8npp"}}}}"#;
        let works: AccountsReceivableResponse = serde_json::from_str(json_data).unwrap();

        let mut expected_blocks = IndexMap::new();
        let mut inner_map = IndexMap::new();
        inner_map.insert(
            BlockHash::zero(),
            SourceInfo {
                amount: Amount::from(1000),
                source: Some(Account::zero()),
                min_version: None,
            },
        );
        expected_blocks.insert(Account::zero(), inner_map);

        let expected_works = AccountsReceivableResponse::Source(AccountsReceivableSource {
            blocks: expected_blocks,
        });

        assert_eq!(works, expected_works);
    }
}
