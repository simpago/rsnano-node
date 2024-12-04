use test_helpers::{setup_rpc_client_and_server, System};

#[test]
fn stop_fails_with_enable_control_disabled() {
    let mut system = System::new();
    let node = system.make_node();

    let server = setup_rpc_client_and_server(node.clone(), false);

    let result = node.runtime.block_on(async { server.client.stop().await });

    assert!(result.is_err());
}
