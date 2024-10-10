use rsnano_ledger::{DEV_GENESIS, DEV_GENESIS_HASH};
use test_helpers::{setup_rpc_client_and_server, System};

#[test]
fn blocks() {
    let mut system = System::new();
    let node = system.make_node();

    let (rpc_client, server) = setup_rpc_client_and_server(node.clone(), false);

    let result = node
        .runtime
        .block_on(async { rpc_client.blocks(vec![*DEV_GENESIS_HASH]).await.unwrap() });

    assert_eq!(
        result.blocks.get(&DEV_GENESIS_HASH).unwrap(),
        &DEV_GENESIS.json_representation()
    );

    server.abort();
}
