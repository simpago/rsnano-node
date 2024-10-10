use rsnano_node::{wallets::WalletsExt, Node};
use rsnano_rpc_messages::{BlockHashRpcMessage, ErrorDto, SendArgs};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn send(node: Arc<Node>, enable_control: bool, args: SendArgs) -> String {
    if enable_control {
        let block_hash =
            node.wallets
                .send_sync(args.wallet, args.source, args.destination, args.amount);
        to_string_pretty(&BlockHashRpcMessage::new("block".to_string(), block_hash)).unwrap()
    } else {
        to_string_pretty(&ErrorDto::new("RPC control is disabled".to_string())).unwrap()
    }
}
