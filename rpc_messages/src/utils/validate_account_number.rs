use crate::{AccountCandidateArg, RpcCommand};

impl RpcCommand {
    pub fn validate_account_number(account: String) -> Self {
        Self::ValidateAccountNumber(AccountCandidateArg { account })
    }
}

#[cfg(test)]
mod tests {
    use crate::RpcCommand;
    use rsnano_core::Account;
    use serde_json::to_string_pretty;

    #[test]
    fn serialize_validate_account_number_command() {
        assert_eq!(
            to_string_pretty(&RpcCommand::validate_account_number(
                Account::zero().encode_account()
            ))
            .unwrap(),
            r#"{
  "action": "validate_account_number",
  "account": "nano_1111111111111111111111111111111111111111111111111111hifc8npp"
}"#
        )
    }

    #[test]
    fn deserialize_validate_account_number_command() {
        let json_str = r#"{
"action": "validate_account_number",
"account": "nano_1111111111111111111111111111111111111111111111111111hifc8npp"
}"#;
        let deserialized: RpcCommand = serde_json::from_str(json_str).unwrap();
        let expected_command =
            RpcCommand::validate_account_number(Account::zero().encode_account());
        assert_eq!(deserialized, expected_command);
    }
}
