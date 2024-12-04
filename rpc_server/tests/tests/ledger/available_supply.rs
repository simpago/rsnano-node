use rsnano_core::Amount;
use test_helpers::{setup_rpc_client_and_server, System};

#[test]
fn available_supply() {
    let mut system = System::new();
    let node = system.make_node();

    let server = setup_rpc_client_and_server(node.clone(), true);

    let result = node
        .runtime
        .block_on(async { server.client.available_supply().await.unwrap() });

    assert_eq!(result.available, Amount::zero());
}
