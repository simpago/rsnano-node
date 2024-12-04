use rsnano_ledger::{DEV_GENESIS_BLOCK, DEV_GENESIS_HASH};
use test_helpers::{setup_rpc_client_and_server, System};

#[test]
fn blocks() {
    let mut system = System::new();
    let node = system.make_node();

    let server = setup_rpc_client_and_server(node.clone(), false);

    let result = node
        .runtime
        .block_on(async { server.client.blocks(vec![*DEV_GENESIS_HASH]).await.unwrap() });

    assert_eq!(
        result.blocks.get(&DEV_GENESIS_HASH).unwrap(),
        &DEV_GENESIS_BLOCK.json_representation()
    );
}
