use crate::{RpcCommand, RpcU64};
use serde::{Deserialize, Serialize};

impl RpcCommand {
    pub fn block_count() -> Self {
        Self::BlockCount
    }
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct BlockCountResponse {
    pub count: RpcU64,
    pub unchecked: RpcU64,
    pub cemented: RpcU64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full: Option<RpcU64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pruned: Option<RpcU64>
}

#[cfg(test)]
mod tests {
    use crate::{ledger::BlockCountResponse, RpcCommand};
    use serde_json::{from_str, to_string_pretty};

    #[test]
    fn serialize_account_block_count_command() {
        assert_eq!(
            serde_json::to_string_pretty(&RpcCommand::BlockCount).unwrap(),
            r#"{
  "action": "block_count"
}"#
        )
    }

    #[test]
    fn derialize_account_block_count_command() {
        let cmd = RpcCommand::BlockCount;
        let serialized = to_string_pretty(&cmd).unwrap();
        let deserialized: RpcCommand = from_str(&serialized).unwrap();
        assert_eq!(cmd, deserialized)
    }

    #[test]
    fn serialize_block_count_dto() {
        let block_count_dto = BlockCountResponse {
            count: 1.into(),
            unchecked: 1.into(),
            cemented: 1.into(),
        };
        assert_eq!(
            serde_json::to_string_pretty(&block_count_dto).unwrap(),
            r#"{
  "count": "1",
  "unchecked": "1",
  "cemented": "1"
}"#
        );
    }

    #[test]
    fn deserialize_block_account_dto() {
        let bool_dto = BlockCountResponse {
            count: 1.into(),
            unchecked: 1.into(),
            cemented: 1.into(),
        };
        let serialized = to_string_pretty(&bool_dto).unwrap();
        let deserialized: BlockCountResponse = from_str(&serialized).unwrap();
        assert_eq!(bool_dto, deserialized);
    }
}
