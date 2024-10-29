use rsnano_core::{Account, Amount};
use serde::{Deserialize, Serialize};

impl AccountsReceivableArgs {
    pub fn new(accounts: Vec<Account>) -> AccountsReceivableArgs {
        Self {
            accounts,
            count: None,
            threshold: None,
            source: None,
            sorting: None,
            include_only_confirmed: None,
            include_active: None,
        }
    }

    pub fn builder(accounts: Vec<Account>) -> AccountsReceivableArgsBuilder {
        AccountsReceivableArgsBuilder {
            args: AccountsReceivableArgs::new(accounts),
        }
    }
}

pub struct AccountsReceivableArgsBuilder {
    args: AccountsReceivableArgs,
}

impl AccountsReceivableArgsBuilder {
    pub fn threshold(mut self, threshold: Amount) -> Self {
        self.args.threshold = Some(threshold);
        self
    }

    pub fn count(mut self, count: u64) -> Self {
        self.args.count = Some(count);
        self
    }

    pub fn include_source(mut self) -> Self {
        self.args.source = Some(true);
        self
    }

    pub fn include_active(mut self, value: bool) -> Self {
        self.args.include_active = Some(value);
        self
    }

    pub fn sorted(mut self) -> Self {
        self.args.sorting = Some(true);
        self
    }

    pub fn only_confirmed(mut self, value: bool) -> Self {
        self.args.include_only_confirmed = Some(value);
        self
    }

    pub fn build(self) -> AccountsReceivableArgs {
        self.args
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AccountsReceivableArgs {
    pub accounts: Vec<Account>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<Amount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sorting: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_only_confirmed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_active: Option<bool>,
}

impl From<Vec<Account>> for AccountsReceivableArgs {
    fn from(value: Vec<Account>) -> Self {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn serialize_accounts_receivable_args_options_some() {
        let args = AccountsReceivableArgs {
            accounts: vec![Account::decode_account(
                "nano_3t6k35gi95xu6tergt6p69ck76ogmitsa8mnijtpxm9fkcm736xtoncuohr3",
            )
            .unwrap()],
            count: Some(10),
            threshold: Some(Amount::raw(1000)),
            source: Some(true),
            sorting: Some(false),
            include_only_confirmed: Some(true),
            include_active: None,
        };

        let serialized = serde_json::to_string(&args).unwrap();

        assert!(serialized.contains(
            "\"accounts\":[\"nano_3t6k35gi95xu6tergt6p69ck76ogmitsa8mnijtpxm9fkcm736xtoncuohr3\"]"
        ));
        assert!(serialized.contains("\"count\":10"));
        assert!(serialized.contains("\"threshold\":\"1000\""));
        assert!(serialized.contains("\"source\":true"));
        assert!(serialized.contains("\"sorting\":false"));
        assert!(serialized.contains("\"include_only_confirmed\":true"));
    }

    #[test]
    fn deserialize_accounts_receivable_args_options_some() {
        let json = r#"{
            "accounts": ["nano_3t6k35gi95xu6tergt6p69ck76ogmitsa8mnijtpxm9fkcm736xtoncuohr3"],
            "count": 5,
            "threshold": "1000",
            "source": true,
            "sorting": false,
            "include_only_confirmed": true,
            "include_active": true
        }"#;

        let deserialized: AccountsReceivableArgs = serde_json::from_str(json).unwrap();

        assert_eq!(deserialized.accounts.len(), 1);
        assert_eq!(
            deserialized.accounts[0],
            Account::decode_account(
                "nano_3t6k35gi95xu6tergt6p69ck76ogmitsa8mnijtpxm9fkcm736xtoncuohr3"
            )
            .unwrap()
        );
        assert_eq!(deserialized.count, Some(5));
        assert_eq!(deserialized.threshold, Some(Amount::raw(1000)));
        assert_eq!(deserialized.source, Some(true));
        assert_eq!(deserialized.sorting, Some(false));
        assert_eq!(deserialized.include_only_confirmed, Some(true));
        assert_eq!(deserialized.include_active, Some(true));
    }

    #[test]
    fn serialize_accounts_receivable_args_options_none() {
        let args = AccountsReceivableArgs {
            accounts: vec![Account::decode_account(
                "nano_3t6k35gi95xu6tergt6p69ck76ogmitsa8mnijtpxm9fkcm736xtoncuohr3",
            )
            .unwrap()],
            count: Some(10),
            threshold: None,
            source: None,
            sorting: None,
            include_only_confirmed: None,
            include_active: None,
        };

        let serialized = serde_json::to_string(&args).unwrap();

        assert!(serialized.contains(
            "\"accounts\":[\"nano_3t6k35gi95xu6tergt6p69ck76ogmitsa8mnijtpxm9fkcm736xtoncuohr3\"]"
        ));
        assert!(serialized.contains("\"count\":10"));
        assert!(!serialized.contains("\"threshold\""));
        assert!(!serialized.contains("\"source\""));
        assert!(!serialized.contains("\"sorting\""));
        assert!(!serialized.contains("\"include_only_confirmed\""));
    }

    #[test]
    fn deserialize_accounts_receivable_args_options_none() {
        let json = r#"{
            "accounts": ["nano_3t6k35gi95xu6tergt6p69ck76ogmitsa8mnijtpxm9fkcm736xtoncuohr3"],
            "count": 5
        }"#;

        let deserialized: AccountsReceivableArgs = serde_json::from_str(json).unwrap();

        assert_eq!(deserialized.accounts.len(), 1);
        assert_eq!(
            deserialized.accounts[0],
            Account::decode_account(
                "nano_3t6k35gi95xu6tergt6p69ck76ogmitsa8mnijtpxm9fkcm736xtoncuohr3"
            )
            .unwrap()
        );
        assert_eq!(deserialized.count, Some(5));
        assert_eq!(deserialized.threshold, None);
        assert_eq!(deserialized.source, None);
        assert_eq!(deserialized.sorting, None);
        assert_eq!(deserialized.include_only_confirmed, None);
    }
}