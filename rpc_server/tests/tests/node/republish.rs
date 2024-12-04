use rsnano_core::{Amount, BlockBuilder, BlockHash, DEV_GENESIS_KEY};
use rsnano_ledger::DEV_GENESIS_HASH;
use rsnano_node::Node;
use rsnano_rpc_messages::RepublishArgs;
use std::{sync::Arc, time::Duration};
use test_helpers::{assert_timely_msg, setup_rpc_client_and_server, System};

fn setup_test_environment(node: Arc<Node>) -> BlockHash {
    let genesis_hash = *DEV_GENESIS_HASH;
    let key = rsnano_core::PrivateKey::new();

    // Create and process send block
    let send = BlockBuilder::legacy_send()
        .previous(genesis_hash)
        .destination(key.public_key().into())
        .balance(Amount::raw(100))
        .sign(DEV_GENESIS_KEY.clone())
        .work(node.work_generate_dev(genesis_hash))
        .build();

    node.process_active(send.clone());
    assert_timely_msg(
        Duration::from_secs(5),
        || node.active.active(&send),
        "send not active on node 1",
    );

    // Create and process open block
    let open = BlockBuilder::legacy_open()
        .source(send.hash())
        .representative(key.public_key().into())
        .account(key.public_key().into())
        .sign(&key)
        .work(node.work_generate_dev(key.public_key()))
        .build();

    node.process_active(open.clone());
    assert_timely_msg(
        Duration::from_secs(5),
        || node.active.active(&open),
        "open not active on node 1",
    );

    open.hash()
}

#[test]
fn test_republish_send_block() {
    let mut system = System::new();
    let node = system.make_node();

    let server = setup_rpc_client_and_server(node.clone(), true);

    setup_test_environment(node.clone());

    let send = node
        .ledger
        .any()
        .get_block(
            &node.store.tx_begin_read(),
            &node
                .ledger
                .any()
                .block_successor(&node.store.tx_begin_read(), &*DEV_GENESIS_HASH)
                .unwrap(),
        )
        .unwrap();

    // Test: Republish send block
    let result = node
        .runtime
        .block_on(async { server.client.republish(send.hash()).await.unwrap() });

    assert_eq!(
        result.blocks.len(),
        1,
        "Expected 1 block, got {}",
        result.blocks.len()
    );
    assert_eq!(result.blocks[0], send.hash(), "Unexpected block hash");

    assert_timely_msg(
        Duration::from_secs(10),
        || {
            node.ledger
                .any()
                .block_exists(&node.ledger.read_txn(), &send.hash())
        },
        "send block not received by node 2",
    );
}

#[test]
fn test_republish_genesis_block() {
    let mut system = System::new();
    let node = system.make_node();

    let server = setup_rpc_client_and_server(node.clone(), true);

    setup_test_environment(node.clone());

    let args = RepublishArgs::builder(*DEV_GENESIS_HASH)
        .with_count(1)
        .build();

    // Test: Republish genesis block with count 1
    let result = node
        .runtime
        .block_on(async { server.client.republish(args).await.unwrap() });

    assert_eq!(
        result.blocks.len(),
        1,
        "Expected 1 block, got {}",
        result.blocks.len()
    );
    assert_eq!(result.blocks[0], *DEV_GENESIS_HASH, "Unexpected block hash");
}

#[test]
fn test_republish_open_block_with_sources() {
    let mut system = System::new();
    let node = system.make_node();

    let server = setup_rpc_client_and_server(node.clone(), true);

    let block_hash = setup_test_environment(node.clone());

    //let genesis_successor = node.ledger.any().block_successor(&node.store.tx_begin_read(), &DEV_GENESIS_HASH).unwrap();
    //let send_successor = node.ledger.any().block_successor(&node.store.tx_begin_read(), &genesis_successor).unwrap();
    //let open = node.ledger.any().get_block(&node.store.tx_begin_read(), &send_successor).unwrap();

    let args = RepublishArgs::builder(block_hash).with_sources(2).build();

    // Test: Republish open block with sources 2
    let result = node
        .runtime
        .block_on(async { server.client.republish(args).await.unwrap() });

    assert_eq!(
        result.blocks.len(),
        3,
        "Expected 3 blocks, got {}",
        result.blocks.len()
    );
    assert_eq!(
        result.blocks[0], *DEV_GENESIS_HASH,
        "Unexpected genesis block hash"
    );
    assert_eq!(
        result.blocks[1],
        node.ledger
            .any()
            .block_successor(&node.store.tx_begin_read(), &*DEV_GENESIS_HASH)
            .unwrap(),
        "Unexpected send block hash"
    );
    assert_eq!(result.blocks[2], block_hash, "Unexpected open block hash");
}
