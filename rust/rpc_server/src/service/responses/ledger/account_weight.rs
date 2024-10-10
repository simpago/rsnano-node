use rsnano_core::Account;
use rsnano_node::Node;
use rsnano_rpc_messages::AmountDto;
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn account_weight(node: Arc<Node>, account: Account) -> String {
    let tx = node.ledger.read_txn();
    let weight = node.ledger.weight_exact(&tx, account.into());
    let account_weight = AmountDto::new("weight".to_string(), weight);
    to_string_pretty(&account_weight).unwrap()
}
