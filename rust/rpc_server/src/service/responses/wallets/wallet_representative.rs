use rsnano_core::WalletId;
use rsnano_node::Node;
use rsnano_rpc_messages::{AccountRpcMessage, ErrorDto};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn wallet_representative(node: Arc<Node>, wallet: WalletId) -> String {
    match node.wallets.get_representative(wallet) {
        Ok(representative) => to_string_pretty(&AccountRpcMessage::new(
            "representative".to_string(),
            representative.into(),
        ))
        .unwrap(),
        Err(e) => to_string_pretty(&ErrorDto::new(e.to_string())).unwrap(),
    }
}
