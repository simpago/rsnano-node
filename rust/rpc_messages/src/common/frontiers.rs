use rsnano_core::{Account, BlockHash};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct FrontiersDto {
    pub frontiers: HashMap<Account, BlockHash>,
}

impl FrontiersDto {
    pub fn new(frontiers: HashMap<Account, BlockHash>) -> Self {
        Self { frontiers }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn serialize_frontiers_dto() {
        let mut frontiers = HashMap::new();
        frontiers.insert(Account::zero(), BlockHash::zero());
        let frontiers_dto = FrontiersDto::new(frontiers);
        let serialized = serde_json::to_string(&frontiers_dto).unwrap();
        let expected_json = r#"{"frontiers":{"nano_1111111111111111111111111111111111111111111111111111hifc8npp":"0000000000000000000000000000000000000000000000000000000000000000"}}"#;
        assert_eq!(serialized, expected_json);
    }

    #[test]
    fn deserialize_frontiers_dto() {
        let json_str = r#"{"frontiers":{"nano_1111111111111111111111111111111111111111111111111111hifc8npp":"0000000000000000000000000000000000000000000000000000000000000000"}}"#;
        let deserialized: FrontiersDto = serde_json::from_str(json_str).unwrap();
        let mut frontiers = HashMap::new();
        frontiers.insert(Account::zero(), BlockHash::zero());
        let expected_error_dto = FrontiersDto::new(frontiers);
        assert_eq!(deserialized, expected_error_dto);
    }
}
