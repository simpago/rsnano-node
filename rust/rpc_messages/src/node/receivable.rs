use crate::RpcCommand;
use rsnano_core::{Account, Amount};
use serde::{Deserialize, Serialize};

impl RpcCommand {
    pub fn receivable(
        account: Account,
        count: u64,
        threshold: Option<Amount>,
        source: Option<bool>,
        min_version: Option<bool>,
        sorting: Option<bool>,
        include_only_confirmed: Option<bool>,
    ) -> Self {
        Self::Receivable(ReceivableArgs {
            account,
            count,
            threshold,
            source,
            min_version,
            sorting,
            include_only_confirmed,
        })
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ReceivableArgs {
    pub account: Account,
    pub count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<Amount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_version: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sorting: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_only_confirmed: Option<bool>,
}

impl ReceivableArgs {
    pub fn new(
        account: Account,
        count: u64,
        threshold: Option<Amount>,
        source: Option<bool>,
        min_version: Option<bool>,
        sorting: Option<bool>,
        include_only_confirmed: Option<bool>,
    ) -> Self {
        Self {
            account,
            count,
            threshold,
            source,
            min_version,
            sorting,
            include_only_confirmed,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::RpcCommand;
    use rsnano_core::Account;
    use serde_json::to_string_pretty;

    #[test]
    fn serialize_receivable_command() {
        assert_eq!(
            to_string_pretty(&RpcCommand::receivable(
                Account::zero(),
                1,
                None,
                None,
                None,
                None,
                None
            ))
            .unwrap(),
            r#"{
  "action": "receivable",
  "account": "nano_1111111111111111111111111111111111111111111111111111hifc8npp",
  "count": 1
}"#
        )
    }

    #[test]
    fn deserialize_receivable_command() {
        let cmd = RpcCommand::receivable(Account::zero(), 1, None, None, None, None, None);
        let serialized = serde_json::to_string_pretty(&cmd).unwrap();
        let deserialized: RpcCommand = serde_json::from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized)
    }
}
