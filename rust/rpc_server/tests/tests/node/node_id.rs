use test_helpers::{setup_rpc_client_and_server, System};

#[test]
fn node_id() {
    let mut system = System::new();
    let node = system.make_node();

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), true);

    node.runtime
        .block_on(async { rpc_client.node_id().await.unwrap() });

    server.abort();
}

#[test]
fn node_id_without_enable_control() {
    let mut system = System::new();
    let node = system.make_node();

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

    let result = node.runtime.block_on(async { rpc_client.node_id().await });

    assert_eq!(
        result.err().map(|e| e.to_string()),
        Some("node returned error: \"RPC control is disabled\"".to_string())
    );

    server.abort();
}
