use rsnano_core::Account;
use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH};
use test_helpers::{setup_rpc_client_and_server, System};

#[test]
fn accounts_frontiers_found() {
    let mut system = System::new();
    let node = system.make_node();

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

    let result = node.runtime.block_on(async {
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

    let result = node.runtime.block_on(async {
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

    let result = node.runtime.block_on(async {
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
