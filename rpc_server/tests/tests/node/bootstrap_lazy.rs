use rsnano_core::BlockHash;
use rsnano_node::config::NodeFlags;
use test_helpers::{send_block, setup_rpc_client_and_server, System};

#[test]
fn bootstrap_any() {
    let mut system = System::new();
    let node = system.make_node();

    let hash = send_block(node.clone());

    let server = setup_rpc_client_and_server(node.clone(), true);

    let result = node
        .runtime
        .block_on(async { server.client.bootstrap_lazy(hash).await.unwrap() });

    assert_eq!(result.started, true.into());
    assert_eq!(result.key_inserted, true.into());
}

#[test]
fn bootstrap_any_fails_with_legacy_bootstrap_disabled() {
    let mut system = System::new();
    let mut flags = NodeFlags::new();
    flags.disable_lazy_bootstrap = true;
    let node = system.build_node().flags(flags).finish();

    let server = setup_rpc_client_and_server(node.clone(), true);

    let result = node
        .runtime
        .block_on(async { server.client.bootstrap_lazy(BlockHash::zero()).await });

    assert_eq!(
        result.err().map(|e| e.to_string()),
        Some("node returned error: \"Lazy bootstrap is disabled\"".to_string())
    );
}
