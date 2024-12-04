#[cfg(test)]
mod tests {
    use crate::{ChainArgs, RpcCommand};
    use rsnano_core::BlockHash;
    use serde_json::{from_value, json};

    #[test]
    fn serialize_successors_command() {
        let block_hash = BlockHash::decode_hex(
            "000D1BAEC8EC208142C99059B393051BAC8380F9B5A2E6B2489A277D81789F3F",
        )
        .unwrap();
        let args = ChainArgs::builder(block_hash, 1)
            .offset(1)
            .reverse()
            .build();
        let successors_command = RpcCommand::Successors(args);

        let serialized = serde_json::to_value(successors_command).unwrap();
        let expected = json!({
            "action": "successors",
            "block": "000D1BAEC8EC208142C99059B393051BAC8380F9B5A2E6B2489A277D81789F3F",
            "count": "1",
            "offset": "1",
            "reverse": "true"
        });

        assert_eq!(serialized, expected);
    }

    #[test]
    fn deserialize_successors_command() {
        let json_value = json!({
            "action": "successors",
            "block": "000D1BAEC8EC208142C99059B393051BAC8380F9B5A2E6B2489A277D81789F3F",
            "count": "1",
            "offset": "1",
            "reverse": "true"
        });

        let deserialized: RpcCommand = from_value(json_value).unwrap();
        let expected_block_hash = BlockHash::decode_hex(
            "000D1BAEC8EC208142C99059B393051BAC8380F9B5A2E6B2489A277D81789F3F",
        )
        .unwrap();
        let args = ChainArgs::builder(expected_block_hash, 1)
            .offset(1)
            .reverse()
            .build();
        let expected = RpcCommand::Successors(args);

        assert_eq!(deserialized, expected);
    }
}
