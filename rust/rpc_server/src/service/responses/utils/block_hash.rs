use rsnano_core::{BlockEnum, JsonBlock};
use rsnano_rpc_messages::BlockHashRpcMessage;
use serde_json::to_string_pretty;

pub async fn block_hash(block: JsonBlock) -> String {
    let block_enum: BlockEnum = block.into();
    to_string_pretty(&BlockHashRpcMessage::new(
        "hash".to_string(),
        block_enum.hash(),
    ))
    .unwrap()
}

#[cfg(test)]
mod tests {
    use rsnano_core::{BlockEnum, BlockHash, StateBlock};
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn block_hash() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

        let block = BlockEnum::State(StateBlock::new_test_instance()).json_representation();

        let result = node
            .tokio
            .block_on(async { rpc_client.block_hash(block).await.unwrap() });

        assert_eq!(
            result.value,
            BlockHash::decode_hex(
                "D9E4A975D8C4E7FE6F3569B6B60EE19D7C090C5B6E316416DC36F8C90264DF60"
            )
            .unwrap()
        );

        server.abort();
    }
}
