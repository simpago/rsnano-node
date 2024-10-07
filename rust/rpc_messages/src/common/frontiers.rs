use rsnano_core::{Account, BlockHash};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct FrontiersDto {
    pub frontiers: HashMap<Account, BlockHash>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<HashMap<Account, String>>,
}

impl FrontiersDto {
    pub fn new(frontiers: HashMap<Account, BlockHash>) -> Self {
        Self { frontiers, errors: None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn serialize_frontiers_dto_with_errors() {
        let mut frontiers = HashMap::new();
        frontiers.insert(Account::decode_account("nano_3t6k35gi95xu6tergt6p69ck76ogmitsa8mnijtpxm9fkcm736xtoncuohr3").unwrap(),
                         BlockHash::decode_hex("023B94B7D27B311666C8636954FE17F1FD2EAA97A8BAC27DE5084FBBD5C6B02C").unwrap());
        
        let mut errors = HashMap::new();
        errors.insert(Account::decode_account("nano_1hrts7hcoozxccnffoq9hqhngnn9jz783usapejm57ejtqcyz9dpso1bibuy").unwrap(),
                      "Account not found".to_string());
        
        let mut frontiers_dto = FrontiersDto::new(frontiers);
        frontiers_dto.errors = Some(errors);
        let serialized = serde_json::to_string_pretty(&frontiers_dto).unwrap();
        let expected_json = r#"{
  "frontiers": {
    "nano_3t6k35gi95xu6tergt6p69ck76ogmitsa8mnijtpxm9fkcm736xtoncuohr3": "023B94B7D27B311666C8636954FE17F1FD2EAA97A8BAC27DE5084FBBD5C6B02C"
  },
  "errors": {
    "nano_1hrts7hcoozxccnffoq9hqhngnn9jz783usapejm57ejtqcyz9dpso1bibuy": "Account not found"
  }
}"#;
        assert_eq!(serialized, expected_json);
    }

    #[test]
    fn serialize_frontiers_dto_without_errors() {
        let mut frontiers = HashMap::new();
        frontiers.insert(Account::decode_account("nano_3t6k35gi95xu6tergt6p69ck76ogmitsa8mnijtpxm9fkcm736xtoncuohr3").unwrap(),
                         BlockHash::decode_hex("023B94B7D27B311666C8636954FE17F1FD2EAA97A8BAC27DE5084FBBD5C6B02C").unwrap());
        
        let frontiers_dto = FrontiersDto::new(frontiers);
        let serialized = serde_json::to_string_pretty(&frontiers_dto).unwrap();
        let expected_json = r#"{
  "frontiers": {
    "nano_3t6k35gi95xu6tergt6p69ck76ogmitsa8mnijtpxm9fkcm736xtoncuohr3": "023B94B7D27B311666C8636954FE17F1FD2EAA97A8BAC27DE5084FBBD5C6B02C"
  }
}"#;
        assert_eq!(serialized, expected_json);
    }

    #[test]
    fn deserialize_frontiers_dto_with_errors() {
        let json_str = r#"{
            "frontiers": {
                "nano_1111111111111111111111111111111111111111111111111111hifc8npp": "0000000000000000000000000000000000000000000000000000000000000000"
            },
            "errors": {
                "nano_3t6k35gi95xu6tergt6p69ck76ogmitsa8mnijtpxm9fkcm736xtoncuohr3": "Account not found"
            }
        }"#;
        let deserialized: FrontiersDto = serde_json::from_str(json_str).unwrap();
        
        let mut frontiers = HashMap::new();
        frontiers.insert(
            Account::decode_account("nano_1111111111111111111111111111111111111111111111111111hifc8npp").unwrap(),
            BlockHash::decode_hex("0000000000000000000000000000000000000000000000000000000000000000").unwrap()
        );
        
        let mut errors = HashMap::new();
        errors.insert(
            Account::decode_account("nano_3t6k35gi95xu6tergt6p69ck76ogmitsa8mnijtpxm9fkcm736xtoncuohr3").unwrap(),
            "Account not found".to_string()
        );
        
        let expected_frontiers_dto = FrontiersDto { frontiers, errors: Some(errors) };
        assert_eq!(deserialized, expected_frontiers_dto);
    }

    #[test]
    fn deserialize_frontiers_dto_without_errors() {
        let json_str = r#"{"frontiers":{"nano_1111111111111111111111111111111111111111111111111111hifc8npp":"0000000000000000000000000000000000000000000000000000000000000000"}}"#;
        let deserialized: FrontiersDto = serde_json::from_str(json_str).unwrap();
        let mut frontiers = HashMap::new();
        frontiers.insert(
            Account::decode_account("nano_1111111111111111111111111111111111111111111111111111hifc8npp").unwrap(),
            BlockHash::decode_hex("0000000000000000000000000000000000000000000000000000000000000000").unwrap()
        );
        let expected_frontiers_dto = FrontiersDto::new(frontiers);
        assert_eq!(deserialized, expected_frontiers_dto);
    }
}
