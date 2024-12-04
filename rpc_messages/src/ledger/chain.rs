use crate::{RpcBool, RpcU64};
use rsnano_core::BlockHash;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChainArgs {
    pub block: BlockHash,
    pub count: RpcU64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<RpcU64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reverse: Option<RpcBool>,
}

impl ChainArgs {
    pub fn new(block: BlockHash, count: u64) -> ChainArgs {
        ChainArgs {
            block,
            count: count.into(),
            offset: None,
            reverse: None,
        }
    }

    pub fn builder(block: BlockHash, count: u64) -> ChainArgsBuilder {
        ChainArgsBuilder::new(block, count)
    }
}

pub struct ChainArgsBuilder {
    args: ChainArgs,
}

impl ChainArgsBuilder {
    fn new(block: BlockHash, count: u64) -> Self {
        Self {
            args: ChainArgs {
                block,
                count: count.into(),
                offset: None,
                reverse: None,
            },
        }
    }

    pub fn offset(mut self, offset: u64) -> Self {
        self.args.offset = Some(offset.into());
        self
    }

    pub fn reverse(mut self) -> Self {
        self.args.reverse = Some(true.into());
        self
    }

    pub fn build(self) -> ChainArgs {
        self.args
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RpcCommand;
    use serde_json::{from_value, json, to_value};

    fn create_test_block_hash() -> BlockHash {
        BlockHash::decode_hex("000D1BAEC8EC208142C99059B393051BAC8380F9B5A2E6B2489A277D81789F3F")
            .unwrap()
    }

    #[test]
    fn test_chain_args_serialize() {
        let block_hash = create_test_block_hash();
        let chain_args = ChainArgs::builder(block_hash, 1).offset(1).build();

        let serialized = serde_json::to_value(chain_args).unwrap();
        let expected = json!({
            "block": "000D1BAEC8EC208142C99059B393051BAC8380F9B5A2E6B2489A277D81789F3F",
            "count": "1",
            "offset": "1"
        });

        assert_eq!(serialized, expected);
    }

    #[test]
    fn deserialize_chain_args() {
        let json_value = json!({
            "block": "000D1BAEC8EC208142C99059B393051BAC8380F9B5A2E6B2489A277D81789F3F",
            "count": "1",
            "offset": "1",
            "reverse": "true"
        });

        let deserialized: ChainArgs = from_value(json_value).unwrap();
        let expected_block_hash = BlockHash::decode_hex(
            "000D1BAEC8EC208142C99059B393051BAC8380F9B5A2E6B2489A277D81789F3F",
        )
        .unwrap();
        let expected = ChainArgs::builder(expected_block_hash, 1)
            .offset(1)
            .reverse()
            .build();

        assert_eq!(deserialized, expected);
    }

    #[test]
    fn serialize_chain_command() {
        let block_hash = BlockHash::decode_hex(
            "000D1BAEC8EC208142C99059B393051BAC8380F9B5A2E6B2489A277D81789F3F",
        )
        .unwrap();
        let args = ChainArgs::builder(block_hash, 1).offset(1).build();
        let chain_command = RpcCommand::Chain(args);

        let serialized = serde_json::to_value(chain_command).unwrap();
        let expected = json!({
            "action": "chain",
            "block": "000D1BAEC8EC208142C99059B393051BAC8380F9B5A2E6B2489A277D81789F3F",
            "count": "1",
            "offset": "1"
        });

        assert_eq!(serialized, expected);
    }

    #[test]
    fn deserialize_chain_command() {
        let json_value = json!({
            "action": "chain",
            "block": "000D1BAEC8EC208142C99059B393051BAC8380F9B5A2E6B2489A277D81789F3F",
            "count": "1",
            "offset": "1"
        });

        let deserialized: RpcCommand = from_value(json_value).unwrap();
        let expected_block_hash = BlockHash::decode_hex(
            "000D1BAEC8EC208142C99059B393051BAC8380F9B5A2E6B2489A277D81789F3F",
        )
        .unwrap();
        let args = ChainArgs::builder(expected_block_hash, 1).offset(1).build();
        let expected = RpcCommand::Chain(args);

        assert_eq!(deserialized, expected);
    }

    #[test]
    fn test_chain_args_builder() {
        let block_hash = BlockHash::decode_hex(
            "000D1BAEC8EC208142C99059B393051BAC8380F9B5A2E6B2489A277D81789F3F",
        )
        .unwrap();

        let chain_args = ChainArgs::builder(block_hash, 1)
            .offset(1)
            .reverse()
            .build();

        let expected = ChainArgs {
            block: block_hash,
            count: 1.into(),
            offset: Some(1.into()),
            reverse: Some(true.into()),
        };
        assert_eq!(chain_args, expected);

        let serialized = to_value(chain_args).unwrap();
        let expected_json = json!({
            "block": "000D1BAEC8EC208142C99059B393051BAC8380F9B5A2E6B2489A277D81789F3F",
            "count": "1",
            "offset": "1",
            "reverse": "true"
        });
        assert_eq!(serialized, expected_json);
    }

    #[test]
    fn test_chain_args_builder_default() {
        let block_hash = BlockHash::decode_hex(
            "000D1BAEC8EC208142C99059B393051BAC8380F9B5A2E6B2489A277D81789F3F",
        )
        .unwrap();

        let chain_args = ChainArgs::builder(block_hash, 1).build();

        let serialized = to_value(chain_args).unwrap();
        let expected_json = json!({
            "block": "000D1BAEC8EC208142C99059B393051BAC8380F9B5A2E6B2489A277D81789F3F",
            "count": "1"
        });
        assert_eq!(serialized, expected_json);
    }
}
