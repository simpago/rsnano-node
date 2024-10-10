use rsnano_ledger::DEV_GENESIS_HASH;
use test_helpers::{setup_rpc_client_and_server, System};

#[test]
fn blocks_info() {
    let mut system = System::new();
    let node = system.make_node();

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

    node.runtime.block_on(async {
        rpc_client
            .blocks_info(vec![*DEV_GENESIS_HASH])
            .await
            .unwrap()
    });

    server.abort();
}
