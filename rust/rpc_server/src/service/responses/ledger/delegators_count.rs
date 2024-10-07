use rsnano_core::Account;
use rsnano_node::node::Node;
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

#[cfg(test)]
mod tests {
    use rsnano_ledger::DEV_GENESIS_ACCOUNT;
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn delegators_count_rpc_response() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let result = node.tokio.block_on(async {
            rpc_client
                .delegators_count(*DEV_GENESIS_ACCOUNT)
                .await
                .unwrap()
        });

        assert_eq!(result.count, 1);

        server.abort();
    }
}
