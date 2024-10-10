use rsnano_core::WalletId;
use rsnano_node::Node;
use rsnano_rpc_messages::{AccountsRpcMessage, ErrorDto};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn account_list(node: Arc<Node>, wallet: WalletId) -> String {
    match node.wallets.get_accounts_of_wallet(&wallet) {
        Ok(accounts) => {
            let account_list = AccountsRpcMessage::new(accounts);
            to_string_pretty(&account_list).unwrap()
        }
        Err(e) => to_string_pretty(&ErrorDto::new(e.to_string())).unwrap(),
    }
}
