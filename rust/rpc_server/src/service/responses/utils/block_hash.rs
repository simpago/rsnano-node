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
