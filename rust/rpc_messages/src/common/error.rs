use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ErrorDto {
    error: String,
}

impl ErrorDto {
    pub fn new(error: String) -> Self {
        Self { error }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn serialize_error_dto() {
        let error_dto = ErrorDto::new("An error occurred".to_string());
        let serialized = serde_json::to_string(&error_dto).unwrap();
        let expected_json = r#"{"error":"An error occurred"}"#;
        assert_eq!(serialized, expected_json);
    }

    #[test]
    fn deserialize_error_dto() {
        let json_str = r#"{"error":"An error occurred"}"#;
        let deserialized: ErrorDto = serde_json::from_str(json_str).unwrap();
        let expected_error_dto = ErrorDto::new("An error occurred".to_string());
        assert_eq!(deserialized, expected_error_dto);
    }
}
