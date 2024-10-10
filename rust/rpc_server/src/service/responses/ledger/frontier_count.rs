use rsnano_node::Node;
use rsnano_rpc_messages::U64RpcMessage;
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn frontier_count(node: Arc<Node>) -> String {
    to_string_pretty(&U64RpcMessage::new(
        "count".to_string(),
        node.ledger.account_count(),
    ))
    .unwrap()
}
