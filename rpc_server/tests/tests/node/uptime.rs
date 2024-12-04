use std::{thread::sleep, time::Duration};
use test_helpers::{setup_rpc_client_and_server, System};

#[test]
fn uptime() {
    let mut system = System::new();
    let node = system.make_node();

    sleep(Duration::from_millis(1000));

    let server = setup_rpc_client_and_server(node.clone(), true);

    let result = node
        .runtime
        .block_on(async { server.client.uptime().await.unwrap() });

    assert!(result.seconds.inner() > 0);
}
