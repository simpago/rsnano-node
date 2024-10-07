use rsnano_core::{Account, BlockHash};
use rsnano_node::node::Node;
use rsnano_rpc_messages::FrontiersDto;
use serde_json::to_string_pretty;
use std::{collections::HashMap, sync::Arc};

pub async fn frontiers(node: Arc<Node>, account: Account, count: u64) -> String {
    let tx = node.ledger.read_txn();
    let mut frontiers: HashMap<Account, BlockHash> = HashMap::new();

    let mut iterator = node.store.account.begin_account(&tx, &account);

    let mut collected = 0;

    while collected < count {
        if let Some((account, account_info)) = iterator.current() {
            frontiers.insert(*account, account_info.head);
            collected += 1;
            iterator.next();
        } else {
            break;
        }
    }

    to_string_pretty(&FrontiersDto::new(frontiers)).unwrap()
}

#[cfg(test)]
mod tests {
    use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH};
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn frontiers() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let result = node
            .tokio
            .block_on(async { rpc_client.frontiers(*DEV_GENESIS_ACCOUNT, 1).await.unwrap() });

        assert_eq!(
            result.frontiers.get(&*DEV_GENESIS_ACCOUNT).unwrap(),
            &*DEV_GENESIS_HASH
        );

        server.abort();
    }
}
