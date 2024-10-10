use crate::RpcCommand;
use rsnano_core::{Account, Amount};
use serde::{Deserialize, Serialize};

impl RpcCommand {
    pub fn delegators(
        account: Account,
        threshold: Option<Amount>,
        count: Option<u64>,
        start: Option<Account>,
    ) -> Self {
        Self::Delegators(DelegatorsArgs::new(account, threshold, count, start))
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct DelegatorsArgs {
    pub account: Account,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<Amount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<Account>,
}

impl DelegatorsArgs {
    pub fn new(
        account: Account,
        threshold: Option<Amount>,
        count: Option<u64>,
        start: Option<Account>,
    ) -> Self {
        Self {
            account,
            threshold,
            count,
            start,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serialize_delegators_command() {
        let command = RpcCommand::delegators(Account::zero(), None, None, None);
        let serialized = serde_json::to_value(command).unwrap();
        let expected = json!({"action": "delegators", "account": "nano_1111111111111111111111111111111111111111111111111111hifc8npp"});
        assert_eq!(serialized, expected);
    }

    #[test]
    fn deserialize_delegators_command() {
        let json = r#"{"action": "delegators","account": "nano_1111111111111111111111111111111111111111111111111111hifc8npp"}"#;
        let deserialized: RpcCommand = serde_json::from_str(json).unwrap();
        let expected = RpcCommand::delegators(Account::zero(), None, None, None);
        assert_eq!(deserialized, expected);
    }

    #[test]
    fn serialize_delegators_args() {
        let args = DelegatorsArgs {
            account: Account::decode_account(
                "nano_1111111111111111111111111111111111111111111111111117353trpda",
            )
            .unwrap(),
            threshold: Some(Amount::raw(1)),
            count: Some(0),
            start: Some(Account::zero()),
        };
        let serialized = serde_json::to_value(args).unwrap();
        let expected = json!({
            "account": "nano_1111111111111111111111111111111111111111111111111117353trpda",
            "threshold": "1",
            "count": 0,
            "start": "nano_1111111111111111111111111111111111111111111111111111hifc8npp"
        });
        assert_eq!(serialized, expected);
    }

    #[test]
    fn deserialize_delegators_args() {
        let json = r#"{
            "account": "nano_1111111111111111111111111111111111111111111111111117353trpda",
            "threshold": "1",
            "count": 0,
            "start": "nano_1111111111111111111111111111111111111111111111111111hifc8npp"
        }"#;
        let deserialized: DelegatorsArgs = serde_json::from_str(json).unwrap();
        assert_eq!(
            deserialized.account,
            Account::decode_account(
                "nano_1111111111111111111111111111111111111111111111111117353trpda"
            )
            .unwrap()
        );
        assert_eq!(deserialized.threshold, Some(Amount::raw(1)));
        assert_eq!(deserialized.count, Some(0));
        assert_eq!(deserialized.start, Some(Account::zero()));
    }
}
