use serde::{Deserialize, Serialize};
use crate::RpcCommand;

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct RepresentativesArgs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sorting: Option<bool>,
}

impl RepresentativesArgs {
    pub fn new(count: Option<u64>, sorting: Option<bool>) -> Self {
        Self { count, sorting }
    }
}

impl RpcCommand {
    pub fn representatives(count: Option<u64>, sorting: Option<bool>) -> Self {
        Self::Representatives(RepresentativesArgs::new(count, sorting))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serialize_representatives_command_options_none() {
        let command = RpcCommand::representatives(None, None);
        let serialized = serde_json::to_value(command).unwrap();
        let expected = json!({"action": "representatives"});
        assert_eq!(serialized, expected);
    }

    #[test]
    fn deserialize_representatives_command_options_none() {
        let json = r#"{"action": "representatives"}"#;
        let deserialized: RpcCommand = serde_json::from_str(json).unwrap();
        let expected = RpcCommand::representatives(None, None);
        assert!(matches!(deserialized, expected));
    }

    #[test]
    fn serialize_representatives_command_options_some() {
        let command = RpcCommand::representatives(Some(10), Some(true));
        let serialized = serde_json::to_value(command).unwrap();
        let expected = json!({
            "action": "representatives",
            "count": 10,
            "sorting": true
        });
        assert_eq!(serialized, expected);
    }

    #[test]
    fn deserialize_representatives_command_options_some() {
        let json = r#"{"action": "representatives", "count": 5, "sorting": false}"#;
        let deserialized: RpcCommand = serde_json::from_str(json).unwrap();
        if let RpcCommand::Representatives(args) = deserialized {
            assert_eq!(args.count, Some(5));
            assert_eq!(args.sorting, Some(false));
        } else {
            panic!("Deserialized to unexpected variant");
        }
    }
}

