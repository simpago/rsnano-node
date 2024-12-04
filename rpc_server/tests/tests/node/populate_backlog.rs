use serde_json::to_string;
use test_helpers::{setup_rpc_client_and_server, System};

#[test]
fn populate_backlog() {
    let mut system = System::new();
    let node = system.make_node();

    let server = setup_rpc_client_and_server(node.clone(), true);

    let result = node
        .runtime
        .block_on(async { server.client.populate_backlog().await.unwrap() });

    assert_eq!(to_string(&result).unwrap(), r#"{"success":""}"#.to_string());
}
