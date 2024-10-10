use rsnano_node::Node;
use rsnano_rpc_messages::SuccessDto;
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn stats_clear(node: Arc<Node>) -> String {
    node.stats.clear();
    to_string_pretty(&SuccessDto::new()).unwrap()
}
