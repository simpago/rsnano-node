use rsnano_core::BlockHash;
use rsnano_node::Node;
use rsnano_rpc_messages::{ErrorDto, SuccessDto};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn work_cancel(node: Arc<Node>, enable_control: bool, hash: BlockHash) -> String {
    if enable_control {
        node.distributed_work.cancel(hash.into());
        to_string_pretty(&SuccessDto::new()).unwrap()
    } else {
        to_string_pretty(&ErrorDto::new("RPC control is disabled".to_string())).unwrap()
    }
}
