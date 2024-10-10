use rsnano_core::Account;
use test_helpers::{setup_rpc_client_and_server, System};

#[test]
fn validate_account_number() {
    let mut system = System::new();
    let node = system.make_node();

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

    node.runtime.block_on(async {
        rpc_client
            .validate_account_number(Account::zero())
            .await
            .unwrap()
    });

    server.abort();
}
