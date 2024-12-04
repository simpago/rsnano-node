use rsnano_ledger::DEV_GENESIS_ACCOUNT;
use test_helpers::{setup_rpc_client_and_server, System};

#[test]
fn delegators_count_rpc_response() {
    let mut system = System::new();
    let node = system.make_node();

    let server = setup_rpc_client_and_server(node.clone(), true);

    let result = node.runtime.block_on(async {
        server
            .client
            .delegators_count(*DEV_GENESIS_ACCOUNT)
            .await
            .unwrap()
    });

    assert_eq!(result.count, 1.into());
}
