use crate::RpcU64;
use rsnano_core::{Account, Amount};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub struct UnopenedArgs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<Account>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<RpcU64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<Amount>,
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct UnopenedResponse {
    pub accounts: HashMap<Account, Amount>,
}

impl UnopenedResponse {
    pub fn new(accounts: HashMap<Account, Amount>) -> Self {
        Self { accounts }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RpcCommand;
    use rsnano_core::Account;
    use serde_json::{from_value, json, to_value};

    #[test]
    fn serialize_unopened_args_threshold_none() {
        let args = UnopenedArgs {
            account: Some(Account::zero()),
            count: Some(1.into()),
            ..Default::default()
        };
        let json = to_value(args).unwrap();

        assert_eq!(
            json,
            json!({
                "account": "nano_1111111111111111111111111111111111111111111111111111hifc8npp",
                "count": "1"
            })
        );
    }

    #[test]
    fn serialize_unopened_args_threshold_some() {
        let args = UnopenedArgs {
            account: Some(Account::zero()),
            count: Some(1.into()),
            threshold: Some(Amount::zero()),
        };
        let json = to_value(args).unwrap();

        assert_eq!(
            json,
            json!({
                "account": "nano_1111111111111111111111111111111111111111111111111111hifc8npp",
                "count": "1",
                "threshold": "0"
            })
        );
    }

    #[test]
    fn serialize_unopened_command_threshold_none() {
        let command = RpcCommand::Unopened(UnopenedArgs {
            account: Some(Account::zero()),
            ..Default::default()
        });
        let json = to_value(command).unwrap();

        assert_eq!(
            json,
            json!({
                "action": "unopened",
                "account": "nano_1111111111111111111111111111111111111111111111111111hifc8npp",
            })
        );
    }

    #[test]
    fn deserialize_unopened_args_threshold_none() {
        let json = json!({
            "account": "nano_1111111111111111111111111111111111111111111111111111hifc8npp",
            "count": "1"
        });

        let args: UnopenedArgs = from_value(json).unwrap();

        assert_eq!(
            args,
            UnopenedArgs {
                account: Some(Account::zero()),
                count: Some(1.into()),
                ..Default::default()
            }
        );
    }

    #[test]
    fn deserialize_unopened_command_threshold_some() {
        let json = json!({
            "action": "unopened",
            "account": "nano_1111111111111111111111111111111111111111111111111111hifc8npp",
            "count": "1",
            "threshold": "0"
        });

        let command: RpcCommand = from_value(json).unwrap();

        assert_eq!(
            command,
            RpcCommand::Unopened(UnopenedArgs {
                account: Some(Account::zero()),
                count: Some(1.into()),
                threshold: Some(Amount::zero())
            })
        );
    }
}
