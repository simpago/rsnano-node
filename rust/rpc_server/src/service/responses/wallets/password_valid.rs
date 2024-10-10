use rsnano_core::WalletId;
use rsnano_node::Node;
use rsnano_rpc_messages::{BoolDto, ErrorDto};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn password_valid(node: Arc<Node>, wallet: WalletId) -> String {
    match node.wallets.valid_password(&wallet) {
        Ok(valid) => to_string_pretty(&BoolDto::new("valid".to_string(), valid)).unwrap(),
        Err(e) => to_string_pretty(&ErrorDto::new(e.to_string())).unwrap(),
    }
}
