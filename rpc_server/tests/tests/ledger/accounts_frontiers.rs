use rsnano_core::Account;
use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH};
use test_helpers::{setup_rpc_client_and_server, System};

#[test]
fn accounts_frontiers_found() {
    let mut system = System::new();
    let node = system.make_node();

    let server = setup_rpc_client_and_server(node.clone(), true);

    let result = node.runtime.block_on(async {
        server
            .client
            .accounts_frontiers(vec![*DEV_GENESIS_ACCOUNT])
            .await
            .unwrap()
    });

    assert_eq!(
        result
            .frontiers
            .unwrap()
            .get(&*DEV_GENESIS_ACCOUNT)
            .unwrap(),
        &*DEV_GENESIS_HASH
    );
}

#[test]
fn accounts_frontiers_account_not_found() {
    let mut system = System::new();
    let node = system.make_node();

    let server = setup_rpc_client_and_server(node.clone(), true);

    let result = node.runtime.block_on(async {
        server
            .client
            .accounts_frontiers(vec![Account::zero()])
            .await
            .unwrap()
    });

    assert_eq!(
        result.errors.unwrap().get(&Account::zero()).unwrap(),
        "Account not found"
    );
}

#[test]
fn accounts_frontiers_found_and_not_found() {
    let mut system = System::new();
    let node = system.make_node();

    let server = setup_rpc_client_and_server(node.clone(), true);

    let result = node.runtime.block_on(async {
        server
            .client
            .accounts_frontiers(vec![*DEV_GENESIS_ACCOUNT, Account::zero()])
            .await
            .unwrap()
    });

    assert_eq!(
        result
            .frontiers
            .as_ref()
            .unwrap()
            .get(&*DEV_GENESIS_ACCOUNT)
            .unwrap(),
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

    assert_eq!(result.frontiers.unwrap().len(), 1);
    assert_eq!(result.errors.as_ref().unwrap().len(), 1);
}
