use test_helpers::{setup_rpc_client_and_server, System};

#[test]
fn block_count() {
    let mut system = System::new();
    let node = system.make_node();

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

    let result = node
        .runtime
        .block_on(async { rpc_client.block_count().await.unwrap() });

    assert_eq!(result.count, 1);
    assert_eq!(result.cemented, 1);
    assert_eq!(result.unchecked, 0);

    server.abort();
}
