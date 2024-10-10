use rsnano_node::Node;
use rsnano_rpc_messages::BlockCountDto;
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn block_count(node: Arc<Node>) -> String {
    let count = node.ledger.block_count();
    let unchecked = node.unchecked.buffer_count() as u64;
    let cemented = node.ledger.cemented_count();
    let block_count = BlockCountDto::new(count, unchecked, cemented);
    to_string_pretty(&block_count).unwrap()
}
