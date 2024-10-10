use rsnano_core::Account;
use rsnano_node::Node;
use rsnano_rpc_messages::{ErrorDto, U64RpcMessage};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn account_block_count(node: Arc<Node>, account: Account) -> String {
    let tx = node.ledger.read_txn();
    match node.ledger.store.account.get(&tx, &account) {
        Some(account_info) => {
            let account_block_count =
                U64RpcMessage::new("block_count".to_string(), account_info.block_count);
            to_string_pretty(&account_block_count).unwrap()
        }
        None => to_string_pretty(&ErrorDto::new("Account not found".to_string())).unwrap(),
    }
}
