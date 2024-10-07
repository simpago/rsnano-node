use rsnano_core::Account;
use rsnano_node::node::Node;
use rsnano_rpc_messages::AmountDto;
use serde_json::to_string_pretty;
use std::sync::Arc;

pub async fn account_weight(node: Arc<Node>, account: Account) -> String {
    let tx = node.ledger.read_txn();
    let weight = node.ledger.weight_exact(&tx, account.into());
    let account_weight = AmountDto::new("weight".to_string(), weight);
    to_string_pretty(&account_weight).unwrap()
}

#[cfg(test)]
mod tests {
    use rsnano_core::Amount;
    use rsnano_ledger::DEV_GENESIS_ACCOUNT;
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn account_weight() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let result = node.tokio.block_on(async {
            rpc_client
                .account_weight(DEV_GENESIS_ACCOUNT.to_owned())
                .await
                .unwrap()
        });

        assert_eq!(result.value, Amount::MAX);

        server.abort();
    }
}
