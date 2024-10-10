use rsnano_core::{Account, WalletId};
use rsnano_node::Node;
use rsnano_rpc_messages::{ErrorDto, SuccessDto};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn wallet_add_watch(
    node: Arc<Node>,
    enable_control: bool,
    wallet: WalletId,
    accounts: Vec<Account>,
) -> String {
    if enable_control {
        match node.wallets.insert_watch(&wallet, &accounts) {
            Ok(_) => to_string_pretty(&SuccessDto::new()).unwrap(),
            Err(e) => to_string_pretty(&ErrorDto::new(e.to_string())).unwrap(),
        }
    } else {
        to_string_pretty(&ErrorDto::new("RPC control is disabled".to_string())).unwrap()
    }
}
