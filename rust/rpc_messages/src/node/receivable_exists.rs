use crate::RpcCommand;
use rsnano_core::BlockHash;
use serde::{Deserialize, Serialize};

impl RpcCommand {
    pub fn receivable_exists(
        hash: BlockHash,
        include_active: Option<bool>,
        include_only_confirmed: Option<bool>,
    ) -> Self {
        Self::ReceivableExists(ReceivableExistsArgs::new(
            hash,
            include_active,
            include_only_confirmed,
        ))
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ReceivableExistsArgs {
    pub hash: BlockHash,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_only_confirmed: Option<bool>,
}

impl ReceivableExistsArgs {
    fn new(
        hash: BlockHash,
        include_active: Option<bool>,
        include_only_confirmed: Option<bool>,
    ) -> Self {
        Self {
            hash,
            include_active,
            include_only_confirmed,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::RpcCommand;
    use rsnano_core::BlockHash;
    use serde_json::to_string_pretty;

    #[test]
    fn serialize_receivable_exists_command_basic() {
        let hash = BlockHash::zero();
        assert_eq!(
            to_string_pretty(&RpcCommand::receivable_exists(hash, None, None)).unwrap(),
            r#"{
  "action": "receivable_exists",
  "hash": "0000000000000000000000000000000000000000000000000000000000000000"
}"#
        )
    }

    #[test]
    fn serialize_receivable_exists_command_with_options() {
        let hash = BlockHash::zero();
        assert_eq!(
            to_string_pretty(&RpcCommand::receivable_exists(
                hash,
                Some(true),
                Some(false)
            ))
            .unwrap(),
            r#"{
  "action": "receivable_exists",
  "hash": "0000000000000000000000000000000000000000000000000000000000000000",
  "include_active": true,
  "include_only_confirmed": false
}"#
        )
    }

    #[test]
    fn deserialize_receivable_exists_command_basic() {
        let hash = BlockHash::zero();
        let cmd = RpcCommand::receivable_exists(hash, None, None);
        let serialized = serde_json::to_string_pretty(&cmd).unwrap();
        let deserialized: RpcCommand = serde_json::from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized)
    }

    #[test]
    fn deserialize_receivable_exists_command_with_options() {
        let hash = BlockHash::zero();
        let cmd = RpcCommand::receivable_exists(hash, Some(true), Some(false));
        let serialized = serde_json::to_string_pretty(&cmd).unwrap();
        let deserialized: RpcCommand = serde_json::from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized)
    }
}
