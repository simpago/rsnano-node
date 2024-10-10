use rsnano_core::Account;
use rsnano_node::Node;
use rsnano_rpc_messages::CountDto;
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn delegators_count(node: Arc<Node>, account: Account) -> String {
    let representative = account;
    let mut count = 0;

    let tx = node.ledger.read_txn();
    let mut iter = node.store.account.begin(&tx);

    while let Some((_, info)) = iter.current() {
        if info.representative == representative.into() {
            count += 1;
        }

        iter.next();
    }
    to_string_pretty(&CountDto::new(count)).unwrap()
}
