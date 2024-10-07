use rsnano_core::Account;
use rsnano_node::node::Node;
use rsnano_rpc_messages::FrontiersDto;
use serde_json::to_string_pretty;
use std::{collections::HashMap, sync::Arc};

pub async fn accounts_frontiers(node: Arc<Node>, accounts: Vec<Account>) -> String {
    let tx = node.ledger.read_txn();
    let mut frontiers = HashMap::new();
    let mut errors = HashMap::new();

    for account in accounts {
        if let Some(block_hash) = node.ledger.any().account_head(&tx, &account) {
            frontiers.insert(account, block_hash);
        } else {
            errors.insert(account, "Account not found".to_string());
        }
    }

    let mut frontiers_dto = FrontiersDto::new(frontiers);
    if !errors.is_empty() {
        frontiers_dto.errors = Some(errors);
    }

    to_string_pretty(&frontiers_dto).unwrap()
}

#[cfg(test)]
mod tests {
    use rsnano_core::Account;
    use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH};
    use test_helpers::{setup_rpc_client_and_server, System};

    #[test]
    fn accounts_frontiers_found() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let result = node.tokio.block_on(async {
            rpc_client
                .accounts_frontiers(vec![*DEV_GENESIS_ACCOUNT])
                .await
                .unwrap()
        });

        assert_eq!(
            result.frontiers.get(&*DEV_GENESIS_ACCOUNT).unwrap(),
            &*DEV_GENESIS_HASH
        );

        server.abort();
    }

    #[test]
    fn accounts_frontiers_account_not_found() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let result = node.tokio.block_on(async {
            rpc_client
                .accounts_frontiers(vec![Account::zero()])
                .await
                .unwrap()
        });

        assert_eq!(
            result.errors.unwrap().get(&Account::zero()).unwrap(),
            "Account not found"
        );

        server.abort();
    }

    #[test]
    fn accounts_frontiers_found_and_not_found() {
        let mut system = System::new();
        let node = system.make_node();

        let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

        let result = node.tokio.block_on(async {
            rpc_client
                .accounts_frontiers(vec![*DEV_GENESIS_ACCOUNT, Account::zero()])
                .await
                .unwrap()
        });

        assert_eq!(
            result.frontiers.get(&*DEV_GENESIS_ACCOUNT).unwrap(),
            &*DEV_GENESIS_HASH
        );

        assert_eq!(
            result
                .errors
                .as_ref()
                .unwrap()
                .get(&Account::zero())
                .unwrap(),
            "Account not found"
        );

        assert_eq!(result.frontiers.len(), 1);
        assert_eq!(result.errors.as_ref().unwrap().len(), 1);

        server.abort();
    }
}
