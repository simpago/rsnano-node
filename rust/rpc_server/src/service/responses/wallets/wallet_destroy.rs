use rsnano_core::WalletId;
use rsnano_node::Node;
use rsnano_rpc_messages::{BoolDto, ErrorDto};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn wallet_destroy(node: Arc<Node>, enable_control: bool, wallet: WalletId) -> String {
    if enable_control {
        node.wallets.destroy(&wallet);
        to_string_pretty(&BoolDto::new("destroyed".to_string(), true)).unwrap()
    } else {
        to_string_pretty(&ErrorDto::new("RPC control is disabled".to_string())).unwrap()
    }
}
