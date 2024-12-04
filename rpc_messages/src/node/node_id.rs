use crate::RpcCommand;

impl RpcCommand {
    pub fn node_id() -> Self {
        Self::NodeId
    }
}

use rsnano_core::{Account, NodeId, PublicKey};
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct NodeIdResponse {
    pub public: PublicKey,
    pub as_account: Account,
    pub node_id: NodeId,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serialize_node_id_command() {
        let command = RpcCommand::node_id();
        let serialized = serde_json::to_value(&command).unwrap();
        let expected = json!({
            "action": "node_id"
        });
        assert_eq!(serialized, expected);
    }

    #[test]
    fn deserialize_node_id_command() {
        let json_str = r#"{"action": "node_id"}"#;
        let deserialized: RpcCommand = serde_json::from_str(json_str).unwrap();
        assert!(matches!(deserialized, RpcCommand::NodeId));
    }

    #[test]
    fn serialize_node_id_dto() {
        let node_id_dto = NodeIdResponse {
            public: PublicKey::zero(),
            as_account: Account::zero(),
            node_id: NodeId::ZERO,
        };

        let serialized = serde_json::to_value(&node_id_dto).unwrap();
        let expected = json!({
            "public": "0000000000000000000000000000000000000000000000000000000000000000",
            "as_account": "nano_1111111111111111111111111111111111111111111111111111hifc8npp",
            "node_id": "node_1111111111111111111111111111111111111111111111111111hifc8npp"
        });

        assert_eq!(serialized, expected);
    }

    #[test]
    fn deserialize_node_id_dto() {
        let json_str = r#"{
            "public": "0000000000000000000000000000000000000000000000000000000000000000",
            "as_account": "nano_1111111111111111111111111111111111111111111111111111hifc8npp",
            "node_id": "node_1111111111111111111111111111111111111111111111111111hifc8npp"
        }"#;

        let deserialized: NodeIdResponse = serde_json::from_str(json_str).unwrap();

        let node_id_dto = NodeIdResponse {
            public: PublicKey::zero(),
            as_account: Account::zero(),
            node_id: NodeId::ZERO,
        };

        assert_eq!(deserialized, node_id_dto);
    }
}
