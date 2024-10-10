use rsnano_node::Node;
use rsnano_rpc_messages::{ErrorDto, NodeIdDto};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn node_id(node: Arc<Node>, enable_control: bool) -> String {
    if enable_control {
        let private = node.node_id.private_key();
        let public = node.node_id.public_key();
        let as_account = public.as_account();

        to_string_pretty(&NodeIdDto::new(private, public, as_account)).unwrap()
    } else {
        to_string_pretty(&ErrorDto::new("RPC control is disabled".to_string())).unwrap()
    }
}
