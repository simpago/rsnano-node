use rsnano_node::Node;
use rsnano_rpc_messages::SuccessDto;
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn populate_backlog(node: Arc<Node>) -> String {
    node.backlog_population.trigger();
    to_string_pretty(&SuccessDto::new()).unwrap()
}
