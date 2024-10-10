use super::chain;
use rsnano_node::Node;
use rsnano_rpc_messages::ChainArgs;
use std::sync::Arc;

pub async fn successors(node: Arc<Node>, args: ChainArgs, successors: bool) -> String {
    chain(node, args, successors).await
}
