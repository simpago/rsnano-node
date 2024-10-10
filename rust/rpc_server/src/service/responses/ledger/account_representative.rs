use rsnano_core::Account;
use rsnano_node::Node;
use rsnano_rpc_messages::{AccountRpcMessage, ErrorDto};
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn account_representative(node: Arc<Node>, account: Account) -> String {
    let tx = node.ledger.read_txn();
    match node.ledger.store.account.get(&tx, &account) {
        Some(account_info) => {
            let account_representative = AccountRpcMessage::new(
                "representative".to_string(),
                account_info.representative.as_account(),
            );
            to_string_pretty(&account_representative).unwrap()
        }
        None => to_string_pretty(&ErrorDto::new("Account not found".to_string())).unwrap(),
    }
}
