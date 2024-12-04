use crate::{BlockSubTypeDto, BlockTypeDto, RpcBool, RpcCommand, RpcU64};
use rsnano_core::{Account, Amount, BlockHash, Link, Signature, WorkNonce};
use serde::{Deserialize, Serialize};

impl RpcCommand {
    pub fn account_history(account_history_args: AccountHistoryArgs) -> Self {
        Self::AccountHistory(account_history_args)
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AccountHistoryArgs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<Account>,
    pub count: RpcU64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<RpcBool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head: Option<BlockHash>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<RpcU64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reverse: Option<RpcBool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_filter: Option<Vec<Account>>,
}

impl AccountHistoryArgs {
    pub fn new(account: Account, count: u64) -> AccountHistoryArgs {
        AccountHistoryArgs {
            account: Some(account),
            count: count.into(),
            raw: None,
            head: None,
            offset: None,
            reverse: None,
            account_filter: None,
        }
    }

    pub fn build_for_account(account: Account, count: u64) -> AccountHistoryArgsBuilder {
        AccountHistoryArgsBuilder::new(Some(account), None, count)
    }

    pub fn build_for_head(head: BlockHash, count: u64) -> AccountHistoryArgsBuilder {
        AccountHistoryArgsBuilder::new(None, Some(head), count)
    }
}

pub struct AccountHistoryArgsBuilder {
    args: AccountHistoryArgs,
}

impl AccountHistoryArgsBuilder {
    fn new(account: Option<Account>, head: Option<BlockHash>, count: u64) -> Self {
        Self {
            args: AccountHistoryArgs {
                account,
                head,
                count: count.into(),
                raw: None,
                offset: None,
                reverse: None,
                account_filter: None,
            },
        }
    }

    pub fn raw(mut self) -> Self {
        self.args.raw = Some(true.into());
        self
    }

    pub fn head(mut self, head: BlockHash) -> Self {
        self.args.head = Some(head);
        self
    }

    pub fn offset(mut self, offset: u64) -> Self {
        self.args.offset = Some(offset.into());
        self
    }

    pub fn reverse(mut self) -> Self {
        self.args.reverse = Some(true.into());
        self
    }

    pub fn account_filter(mut self, account_filter: Vec<Account>) -> Self {
        self.args.account_filter = Some(account_filter);
        self
    }

    pub fn finish(self) -> AccountHistoryArgs {
        self.args
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AccountHistoryResponse {
    pub account: Account,
    pub history: Vec<HistoryEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous: Option<BlockHash>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<BlockHash>,
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub local_timestamp: RpcU64,
    pub height: RpcU64,
    pub hash: BlockHash,
    pub confirmed: RpcBool,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub block_type: Option<BlockTypeDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtype: Option<BlockSubTypeDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<Account>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_account: Option<Account>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<Amount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work: Option<WorkNonce>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<Signature>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub representative: Option<Account>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous: Option<BlockHash>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub balance: Option<Amount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<BlockHash>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opened: Option<Account>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<Account>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<Link>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::to_string_pretty;

    #[test]
    fn serialize_account_history_command() {
        let account_history_args = AccountHistoryArgs::build_for_head(BlockHash::zero(), 10)
            .raw()
            .offset(5)
            .reverse()
            .account_filter(vec![Account::from(123)])
            .finish();

        assert_eq!(
            to_string_pretty(&RpcCommand::account_history(account_history_args)).unwrap(),
            r#"{
  "action": "account_history",
  "count": "10",
  "raw": "true",
  "head": "0000000000000000000000000000000000000000000000000000000000000000",
  "offset": "5",
  "reverse": "true",
  "account_filter": [
    "nano_111111111111111111111111111111111111111111111111115uwdgas549"
  ]
}"#
        )
    }

    #[test]
    fn deserialize_account_history_command() {
        let json = r#"{
            "action": "account_history",
            "account": "nano_111111111111111111111111111111111111111111111111115uwdgas549",
            "count": "5",
            "raw": "true",
            "head": "0000000000000000000000000000000000000000000000000000000000000000",
            "offset": "10",
            "reverse": "false",
            "account_filter": ["nano_1111111111111111111111111111111111111111111111111111hifc8npp"]
        }"#;

        let deserialized: RpcCommand = serde_json::from_str(json).unwrap();

        if let RpcCommand::AccountHistory(args) = deserialized {
            assert_eq!(args.account, Some(Account::from(123)));
            assert_eq!(args.head, Some(BlockHash::zero()));
            assert_eq!(args.count, 5.into());
            assert_eq!(args.raw, Some(true.into()));
            assert_eq!(args.head, Some(BlockHash::zero()));
            assert_eq!(args.offset, Some(10.into()));
            assert_eq!(args.reverse, Some(false.into()));
            assert_eq!(args.account_filter, Some(vec![Account::zero()]));
        } else {
            panic!("Deserialized to wrong RpcCommand variant");
        }
    }
}
