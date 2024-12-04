use rsnano_ledger::DEV_GENESIS_ACCOUNT;
use rsnano_rpc_messages::AccountsRepresentativesResponse;
use std::collections::HashMap;
use test_helpers::{setup_rpc_client_and_server, System};

#[test]
fn accounts_representatives() {
    let mut system = System::new();
    let node = system.make_node();

    let server = setup_rpc_client_and_server(node.clone(), true);

    let result = node.runtime.block_on(async {
        server
            .client
            .accounts_representatives(vec![*DEV_GENESIS_ACCOUNT])
            .await
            .unwrap()
    });

    let mut accounts_representatives = HashMap::new();
    accounts_representatives.insert(*DEV_GENESIS_ACCOUNT, *DEV_GENESIS_ACCOUNT);

    let expected = AccountsRepresentativesResponse {
        representatives: Some(accounts_representatives),
        errors: None,
    };
    assert_eq!(result, expected);
}
