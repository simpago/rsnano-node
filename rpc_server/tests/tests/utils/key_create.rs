use test_helpers::{setup_rpc_client_and_server, System};

#[test]
fn key_create() {
    let mut system = System::new();
    let node = system.make_node();

    let server = setup_rpc_client_and_server(node.clone(), false);

    node.runtime
        .block_on(async { server.client.key_create().await.unwrap() });
}
