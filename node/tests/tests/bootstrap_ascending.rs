use std::time::Duration;

use rsnano_core::{
    work::{WorkPool, WorkPoolImpl},
    Amount, Block, BlockHash, KeyPair, StateBlock, DEV_GENESIS_KEY,
};
use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH, DEV_GENESIS_PUB_KEY};
use test_helpers::{assert_timely, System};

/**
 * Tests the base case for returning
 */
#[test]
fn account_base() {
    let mut system = System::new();
    let node0 = system.make_node();
    let send1 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::raw(1),
        0.into(),
        &DEV_GENESIS_KEY,
        node0.work_generate_dev(*DEV_GENESIS_HASH),
    ));
    node0.process(send1.clone()).unwrap();
    let node1 = system.make_node();
    assert_timely(Duration::from_secs(5), || node1.block_exists(&send1.hash()));
}

/**
 * Tests that bootstrap_ascending will return multiple new blocks in-order
 */
#[test]
fn account_inductive() {
    let mut system = System::new();
    let node0 = system.make_node();
    let send1 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::raw(1),
        0.into(),
        &DEV_GENESIS_KEY,
        node0.work_generate_dev(*DEV_GENESIS_HASH),
    ));
    let send2 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        send1.hash(),
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::raw(2),
        0.into(),
        &DEV_GENESIS_KEY,
        node0.work_generate_dev(send1.hash()),
    ));
    node0.process(send1).unwrap();
    node0.process(send2.clone()).unwrap();
    let node1 = system.make_node();
    assert_timely(Duration::from_secs(50), || {
        node1.block_exists(&send2.hash())
    });
}

/**
 * Tests that bootstrap_ascending will return multiple new blocks in-order
 */

#[test]
fn trace_base() {
    let mut system = System::new();
    let node0 = system.make_node();
    let key = KeyPair::new();
    let send1 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::raw(1),
        key.public_key().as_account().into(),
        &DEV_GENESIS_KEY,
        node0.work_generate_dev(*DEV_GENESIS_HASH),
    ));
    let receive1 = Block::State(StateBlock::new(
        key.public_key().as_account(),
        BlockHash::zero(),
        *DEV_GENESIS_PUB_KEY,
        Amount::raw(1),
        send1.hash().into(),
        &key,
        node0.work_generate_dev(&key),
    ));
    node0.process(send1).unwrap();
    node0.process(receive1.clone()).unwrap();
    let node1 = system.make_node();
    assert_timely(Duration::from_secs(10), || {
        node1.block_exists(&receive1.hash())
    });
}
