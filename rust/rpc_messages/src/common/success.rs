use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct SuccessDto {
    success: String,
}

impl SuccessDto {
    pub fn new() -> Self {
        Self {
            success: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn serialize_success_dto() {
        let success_dto = SuccessDto::new();
        let serialized = serde_json::to_string(&success_dto).unwrap();
        let expected_json = r#"{"success":""}"#;
        assert_eq!(serialized, expected_json);
    }

    #[test]
    fn deserialize_success_dto() {
        let json_str = r#"{"success":""}"#;
        let deserialized: SuccessDto = serde_json::from_str(json_str).unwrap();
        let expected_error_dto = SuccessDto::new();
        assert_eq!(deserialized, expected_error_dto);
    }
}
