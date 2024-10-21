use crate::RpcCommand;
use rsnano_core::{Account, Amount};
use serde::{Deserialize, Serialize};

impl RpcCommand {
    pub fn delegators(args: DelegatorsArgs) -> Self {
        Self::Delegators(args)
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
    pub fn new(account: Account) -> DelegatorsArgs {
        DelegatorsArgs {
            account,
            threshold: None,
            count: None,
            start: None,
        }
    }

    pub fn builder(account: Account) -> DelegatorsArgsBuilder {
        DelegatorsArgsBuilder::new(account)
    }
}

pub struct DelegatorsArgsBuilder {
    args: DelegatorsArgs,
}

impl DelegatorsArgsBuilder {
    fn new(account: Account) -> Self {
        Self {
            args: DelegatorsArgs {
                account,
                threshold: None,
                count: None,
                start: None,
            },
        }
    }

    pub fn with_minimum_balance(mut self, threshold: Amount) -> Self {
        self.args.threshold = Some(threshold);
        self
    }

    pub fn count(mut self, count: u64) -> Self {
        self.args.count = Some(count);
        self
    }

    pub fn start_from(mut self, start: Account) -> Self {
        self.args.start = Some(start);
        self
    }

    pub fn build(self) -> DelegatorsArgs {
        self.args
    }
}

impl From<Account> for DelegatorsArgs {
    fn from(account: Account) -> Self {
        Self {
            account,
            threshold: None,
            count: None,
            start: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serialize_delegators_command() {
        let command = RpcCommand::delegators(Account::zero().into());
        let serialized = serde_json::to_value(command).unwrap();
        let expected = json!({"action": "delegators", "account": "nano_1111111111111111111111111111111111111111111111111111hifc8npp"});
        assert_eq!(serialized, expected);
    }

    #[test]
    fn deserialize_delegators_command() {
        let json = r#"{"action": "delegators","account": "nano_1111111111111111111111111111111111111111111111111111hifc8npp"}"#;
        let deserialized: RpcCommand = serde_json::from_str(json).unwrap();
        let expected = RpcCommand::delegators(Account::zero().into());
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

    #[test]
    fn test_delegators_args_builder() {
        let args = DelegatorsArgs::builder(Account::zero())
            .with_minimum_balance(Amount::raw(1000))
            .count(50)
            .start_from(Account::from(123))
            .build();

        assert_eq!(args.account, Account::zero());
        assert_eq!(args.threshold, Some(Amount::raw(1000)));
        assert_eq!(args.count, Some(50));
        assert_eq!(args.start, Some(Account::from(123)));
    }

    #[test]
    fn test_delegators_args_builder_partial() {
        let args = DelegatorsArgs::builder(Account::zero()).count(30).build();

        assert_eq!(args.account, Account::zero());
        assert_eq!(args.threshold, None);
        assert_eq!(args.count, Some(30));
        assert_eq!(args.start, None);
    }

    #[test]
    fn test_delegators_args_from_account() {
        let account = Account::from(123);
        let args: DelegatorsArgs = account.into();

        assert_eq!(args.account, account);
        assert_eq!(args.threshold, None);
        assert_eq!(args.count, None);
        assert_eq!(args.start, None);
    }
}
