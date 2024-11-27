use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
    time::Duration,
};

use rsnano_core::{Amount, Block, KeyPair, StateBlock, DEV_GENESIS_KEY};
use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH, DEV_GENESIS_PUB_KEY};
use test_helpers::{assert_timely, assert_timely_eq, setup_independent_blocks, System};

/*
 * Ensures all not confirmed accounts get activated by backlog scan periodically
 */
#[test]
fn backlog_population() {
    let activated = Arc::new(Mutex::new(HashSet::new()));
    let activated2 = activated.clone();
    let mut system = System::new();
    let node = system.make_node();

    node.backlog_population
        .set_activate_callback(Box::new(move |_tx, account| {
            activated2.lock().unwrap().insert(*account);
        }));

    let blocks = setup_independent_blocks(&node, 256, &DEV_GENESIS_KEY);

    // Checks if `activated` set contains all accounts we previously set up
    assert_timely(Duration::from_secs(5), || {
        let guard = activated.lock().unwrap();
        blocks.iter().all(|b| guard.contains(&b.account()))
    });

    // Clear activated set to ensure we activate those accounts more than once
    activated.lock().unwrap().clear();

    assert_timely(Duration::from_secs(5), || {
        let guard = activated.lock().unwrap();
        blocks.iter().all(|b| guard.contains(&b.account()))
    });
}

#[test]
fn election_activation() {
    let key = KeyPair::new();
    let mut system = System::new();
    let node = system.build_node().finish();
    let send = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::nano(1000),
        key.public_key().as_account().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(*DEV_GENESIS_HASH),
    ));
    node.process(send).unwrap();
    assert_timely_eq(Duration::from_secs(5), || node.active.len(), 1);
}
