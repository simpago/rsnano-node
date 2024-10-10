use rsnano_core::WalletId;
use rsnano_node::Node;
use rsnano_rpc_messages::{BoolDto, ErrorDto};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn wallet_lock(node: Arc<Node>, enable_control: bool, wallet: WalletId) -> String {
    if enable_control {
        match node.wallets.lock(&wallet) {
            Ok(()) => to_string_pretty(&BoolDto::new("locked".to_string(), true)).unwrap(),
            Err(e) => to_string_pretty(&ErrorDto::new(e.to_string())).unwrap(),
        }
    } else {
        to_string_pretty(&ErrorDto::new("RPC control is disabled".to_string())).unwrap()
    }
}
