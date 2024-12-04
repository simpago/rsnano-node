use crate::RpcBoolNumber;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct SuccessResponse {
    success: String,
}

impl SuccessResponse {
    pub fn new() -> Self {
        Self {
            success: String::new(),
        }
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ChangedResponse {
    changed: RpcBoolNumber,
}

impl ChangedResponse {
    pub fn new(changed: bool) -> Self {
        Self {
            changed: changed.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn serialize_success_dto() {
        let success_dto = SuccessResponse::new();
        let serialized = serde_json::to_string(&success_dto).unwrap();
        let expected_json = r#"{"success":""}"#;
        assert_eq!(serialized, expected_json);
    }

    #[test]
    fn deserialize_success_dto() {
        let json_str = r#"{"success":""}"#;
        let deserialized: SuccessResponse = serde_json::from_str(json_str).unwrap();
        let expected_error_dto = SuccessResponse::new();
        assert_eq!(deserialized, expected_error_dto);
    }
}
