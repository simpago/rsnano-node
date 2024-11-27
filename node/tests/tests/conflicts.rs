use rsnano_core::{Amount, Block, BlockSideband, KeyPair, StateBlock, DEV_GENESIS_KEY};
use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH, DEV_GENESIS_PUB_KEY};
use rsnano_network::ChannelId;
use rsnano_node::block_processing::BlockSource;
use std::time::Duration;
use test_helpers::{
    assert_timely, assert_timely_eq, setup_new_account, start_election, start_elections, System,
};

#[test]
fn start_stop() {
    let mut system = System::new();
    let node1 = system.make_node();
    let key1 = KeyPair::new();
    let send1 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::zero(),
        key1.public_key().as_account().into(),
        &DEV_GENESIS_KEY,
        node1.work_generate_dev(*DEV_GENESIS_HASH),
    ));
    node1.process(send1.clone()).unwrap();
    assert_eq!(node1.active.len(), 0);
    let election1 = start_election(&node1, &send1.hash());
    assert_eq!(node1.active.len(), 1);
    assert_eq!(election1.vote_count(), 1);
}

#[test]
fn add_existing() {
    let mut system = System::new();
    let node1 = system.make_node();
    let key1 = KeyPair::new();

    // create a send block to send all of the nano supply to key1
    let send1 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::zero(),
        key1.public_key().as_account().into(),
        &DEV_GENESIS_KEY,
        node1.work_generate_dev(*DEV_GENESIS_HASH),
    ));

    // add the block to ledger as an unconfirmed block
    node1.process(send1.clone()).unwrap();

    // instruct the election scheduler to trigger an election for send1
    start_election(&node1, &send1.hash());

    // wait for election to be started before processing send2
    assert_timely(Duration::from_secs(5), || {
        node1.active.active(&send1)
    });

    let key2 = KeyPair::new();
    let mut send2 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::zero(),
        key2.public_key().as_account().into(),
        &DEV_GENESIS_KEY,
        node1.work_generate_dev(*DEV_GENESIS_HASH),
    ));
    send2.set_sideband(BlockSideband::new_test_instance());

    // the block processor will notice that the block is a fork and it will try to publish it
    // which will update the election object
    node1
        .block_processor
        .add(send2.clone().into(), BlockSource::Live, ChannelId::LOOPBACK);

    assert!(node1.active.active(&send1));
    assert_timely(Duration::from_secs(5), || {
        node1.active.active(&send2)
    });
}

#[test]
fn add_two() {
    let mut system = System::new();
    let node = system.make_node();
    let key1 = KeyPair::new();
    let key2 = KeyPair::new();
    let key3 = KeyPair::new();
    let gk = DEV_GENESIS_KEY.clone();

    // create 2 new accounts, that receive 1 raw each, all blocks are force confirmed
    let (_send1, open1) =
        setup_new_account(&node, Amount::raw(1), &gk, &key1, gk.public_key(), true);
    let (_send2, open2) =
        setup_new_account(&node, Amount::raw(1), &gk, &key2, gk.public_key(), true);
    assert_eq!(node.ledger.cemented_count(), 5);

    // send 1 raw to account key3 from key1
    let send_a = Block::State(StateBlock::new(
        key1.public_key().as_account(),
        open1.hash(),
        *DEV_GENESIS_PUB_KEY,
        Amount::zero(),
        key3.public_key().as_account().into(),
        &key1,
        node.work_generate_dev(open1.hash()),
    ));

    // send 1 raw to account key3 from key2
    let send_b = Block::State(StateBlock::new(
        key2.public_key().as_account(),
        open2.hash(),
        *DEV_GENESIS_PUB_KEY,
        Amount::zero(),
        key3.public_key().as_account().into(),
        &key2,
        node.work_generate_dev(open2.hash()),
    ));

    // activate elections for the previous two send blocks (to account3) that we did not forcefully confirm
    node.process(send_a.clone()).unwrap();
    node.process(send_b.clone()).unwrap();
    start_elections(&node, &[send_a.hash(), send_b.hash()], false);
    assert!(node
        .active
        .election(&send_a.qualified_root())
        .is_some());
    assert!(node
        .active
        .election(&send_b.qualified_root())
        .is_some());
    assert_timely_eq(Duration::from_secs(5), || node.active.len(), 2);
}
