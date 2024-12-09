use crate::{RpcBool, RpcCommand, RpcU64};
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct RepresentativesArgs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<RpcU64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sorting: Option<RpcBool>,
}

impl RpcCommand {
    pub fn representatives() -> Self {
        Self::Representatives(RepresentativesArgs {
            count: None,
            sorting: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serialize_representatives_command_options_none() {
        let command = RpcCommand::representatives();
        let serialized = serde_json::to_value(command).unwrap();
        let expected = json!({"action": "representatives"});
        assert_eq!(serialized, expected);
    }

    #[test]
    fn deserialize_representatives_command_options_none() {
        let json = r#"{"action": "representatives"}"#;
        let deserialized: RpcCommand = serde_json::from_str(json).unwrap();
        let expected = RpcCommand::representatives();
        assert_eq!(deserialized, expected);
    }

    #[test]
    fn serialize_representatives_command_options_some() {
        let command = RpcCommand::Representatives(RepresentativesArgs {
            count: Some(10.into()),
            sorting: Some(true.into()),
        });
        let serialized = serde_json::to_value(command).unwrap();
        let expected = json!({
            "action": "representatives",
            "count": "10",
            "sorting": "true"
        });
        assert_eq!(serialized, expected);
    }

    #[test]
    fn deserialize_representatives_command_options_some() {
        let json = r#"{"action": "representatives", "count": "5", "sorting": "false"}"#;
        let deserialized: RpcCommand = serde_json::from_str(json).unwrap();
        if let RpcCommand::Representatives(args) = deserialized {
            assert_eq!(args.count, Some(5.into()));
            assert_eq!(args.sorting, Some(false.into()));
        } else {
            panic!("Deserialized to unexpected variant");
        }
    }
}
