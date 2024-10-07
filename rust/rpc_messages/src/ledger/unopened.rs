use rsnano_core::{Account, Amount};
use serde::{Deserialize, Serialize};
use crate::RpcCommand;

impl RpcCommand {
    pub fn unopened(account: Account, count: u64, threshold: Option<Amount>) -> Self {
        Self::Unopened(UnopenedArgs::new(account, count, threshold))
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct UnopenedArgs {
    pub account: Account,
    pub count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<Amount>
}

impl UnopenedArgs {
    pub fn new(account: Account, count: u64, threshold: Option<Amount>) -> Self {
        Self { account, count, threshold }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, from_value, to_value};

    #[test]
    fn serialize_unopened_args_threshold_none() {
        let args = UnopenedArgs::new(Account::zero(), 1, None);
        let json = to_value(args).unwrap();
        
        assert_eq!(json, json!({
            "account": "nano_1111111111111111111111111111111111111111111111111111hifc8npp",
            "count": 1
        }));
    }

    #[test]
    fn serialize_unopened_args_threshold_some() {
        let args = UnopenedArgs::new(Account::zero(), 1, Some(Amount::zero()));
        let json = to_value(args).unwrap();
        
        assert_eq!(json, json!({
            "account": "nano_1111111111111111111111111111111111111111111111111111hifc8npp",
            "count": 1,
            "threshold": "0"
        }));
    }

    #[test]
    fn serialize_unopened_command_threshold_none() {
        let command = RpcCommand::unopened(Account::zero(), 1, None);
        let json = to_value(command).unwrap();
        
        assert_eq!(json, json!({
            "action": "unopened",
            "account": "nano_1111111111111111111111111111111111111111111111111111hifc8npp",
            "count": 1
        }));
    }

    #[test]
    fn serialize_unopened_command_threshold_some() {
        let command = RpcCommand::unopened(Account::zero(), 1, Some(Amount::zero()));
        let json = to_value(command).unwrap();
        
        assert_eq!(json, json!({
            "action": "unopened",
            "account": "nano_1111111111111111111111111111111111111111111111111111hifc8npp",
            "count": 1,
            "threshold": "0"
        }));
    }

    #[test]
    fn deserialize_unopened_args_threshold_none() {
        let json = json!({
            "account": "nano_1111111111111111111111111111111111111111111111111111hifc8npp",
            "count": 1
        });

        let args: UnopenedArgs = from_value(json).unwrap();

        assert_eq!(args, UnopenedArgs::new(Account::zero(), 1, None));
    }

    #[test]
    fn deserialize_unopened_args_threshold_some() {
        let json = json!({
            "account": "nano_1111111111111111111111111111111111111111111111111111hifc8npp",
            "count": 1,
            "threshold": "0"
        });

        let args: UnopenedArgs = from_value(json).unwrap();

        assert_eq!(args, UnopenedArgs::new(Account::zero(), 1, Some(Amount::zero())));
    }

    #[test]
    fn deserialize_unopened_command_threshold_none() {
        let json = json!({
            "action": "unopened",
            "account": "nano_1111111111111111111111111111111111111111111111111111hifc8npp",
            "count": 1
        });

        let command: RpcCommand = from_value(json).unwrap();

        assert_eq!(command, RpcCommand::unopened(Account::zero(), 1, None));
    }

    #[test]
    fn deserialize_unopened_command_threshold_some() {
        let json = json!({
            "action": "unopened",
            "account": "nano_1111111111111111111111111111111111111111111111111111hifc8npp",
            "count": 1,
            "threshold": "0"
        });

        let command: RpcCommand = from_value(json).unwrap();

        assert_eq!(command, RpcCommand::unopened(Account::zero(), 1, Some(Amount::zero())));
    }
}