use rsnano_node::stats::{DetailType, Direction, StatType};
use std::time::Duration;
use test_helpers::{setup_rpc_client_and_server, System};

#[test]
fn stats_clear() {
    let mut system = System::new();
    let node = system.make_node();

    let server = setup_rpc_client_and_server(node.clone(), true);

    node.runtime
        .block_on(async { server.client.stats_clear().await.unwrap() });

    assert_eq!(
        node.stats
            .count(StatType::Ledger, DetailType::Fork, Direction::In),
        0
    );

    assert!(node.stats.last_reset() <= Duration::from_secs(5));
}
