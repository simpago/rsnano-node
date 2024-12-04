use rsnano_core::{Amount, Block, StateBlock, DEV_GENESIS_KEY};
use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH, DEV_GENESIS_PUB_KEY};
use rsnano_rpc_messages::{BlockSubTypeDto, ProcessArgs};
use test_helpers::{setup_rpc_client_and_server, System};

#[test]
fn process() {
    let mut system = System::new();
    let node = system.make_node();

    let server = setup_rpc_client_and_server(node.clone(), false);

    let send1 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::raw(100),
        DEV_GENESIS_KEY.account().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(*DEV_GENESIS_HASH),
    ));

    let args: ProcessArgs = ProcessArgs::build(send1.json_representation())
        .subtype(BlockSubTypeDto::Send)
        .finish();

    let result = node
        .runtime
        .block_on(async { server.client.process(args).await.unwrap() });

    assert_eq!(result.hash, send1.hash());
    assert_eq!(node.latest(&*DEV_GENESIS_ACCOUNT), send1.hash());
}

#[test]
fn process_fails_with_low_work() {
    let mut system = System::new();
    let node = system.make_node();

    let server = setup_rpc_client_and_server(node.clone(), false);

    let send1 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::raw(100),
        DEV_GENESIS_KEY.account().into(),
        &DEV_GENESIS_KEY,
        1,
    ));

    let args: ProcessArgs = ProcessArgs::build(send1.json_representation())
        .subtype(BlockSubTypeDto::Send)
        .finish();

    let result = node
        .runtime
        .block_on(async { server.client.process(args).await });

    assert_eq!(
        result.err().map(|e| e.to_string()),
        Some("node returned error: \"Block work is less than threshold\"".to_string())
    );
}
