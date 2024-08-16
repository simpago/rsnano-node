use std::{sync::Arc, time::Duration};

use super::helpers::{assert_timely, assert_timely_eq, get_available_port, System};
use rsnano_core::{
    Amount, BlockEnum, BlockHash, KeyPair, StateBlock, Vote, VoteSource, DEV_GENESIS_KEY,
};
use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH};
use rsnano_node::{
    config::{FrontiersConfirmationMode, NodeConfig},
    wallets::WalletsExt,
};

// FIXME: this test fails on rare occasions. It needs a review.
#[test]
fn quorum_minimum_update_weight_before_quorum_checks() {
    let mut system = System::new();
    let config = NodeConfig {
        frontiers_confirmation: FrontiersConfirmationMode::Disabled,
        ..System::default_config()
    };
    let node1 = system.build_node().config(config.clone()).finish();
    let wallet_id1 = node1.wallets.wallet_ids()[0];
    node1
        .wallets
        .insert_adhoc2(&wallet_id1, &DEV_GENESIS_KEY.private_key(), true)
        .unwrap();

    let key1 = KeyPair::new();
    let amount = (config.online_weight_minimum / 100
        * node1.online_reps.lock().unwrap().quorum_percent() as u128)
        - Amount::raw(1);

    let send1 = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_ACCOUNT,
        amount,
        key1.public_key().into(),
        &DEV_GENESIS_KEY,
        node1.work_generate_dev((*DEV_GENESIS_HASH).into()),
    ));
    node1.process_active(send1.clone());
    assert_timely(Duration::from_secs(5), || {
        node1.block(&send1.hash()).is_some()
    });

    let open1 = BlockEnum::State(StateBlock::new(
        key1.public_key(),
        BlockHash::zero(),
        key1.public_key(),
        Amount::MAX - amount,
        send1.hash().into(),
        &key1,
        node1.work_generate_dev(key1.public_key().into()),
    ));
    node1.process(open1.clone()).unwrap();

    let key2 = KeyPair::new();
    let send2 = BlockEnum::State(StateBlock::new(
        key1.public_key(),
        open1.hash(),
        key1.public_key(),
        Amount::raw(3),
        key2.public_key().into(),
        &key1,
        node1.work_generate_dev(open1.hash().into()),
    ));
    node1.process(send2.clone()).unwrap();
    assert_timely_eq(Duration::from_secs(5), || node1.ledger.block_count(), 4);

    let config2 = NodeConfig {
        peering_port: Some(get_available_port()),
        ..config
    };
    let node2 = system.build_node().config(config2).finish();
    let wallet_id2 = node2.wallets.wallet_ids()[0];
    node2
        .wallets
        .insert_adhoc2(&wallet_id2, &key1.private_key(), true)
        .unwrap();
    assert_timely_eq(Duration::from_secs(15), || node2.ledger.block_count(), 4);

    assert_timely(Duration::from_secs(5), || {
        node1.active.election(&send1.qualified_root()).is_some()
    });
    let election = node1.active.election(&send1.qualified_root()).unwrap();
    assert_eq!(1, election.mutex.lock().unwrap().last_blocks.len());

    let vote1 = Arc::new(Vote::new_final(&DEV_GENESIS_KEY, vec![send1.hash()]));
    node1.vote_router.vote(&vote1, VoteSource::Live);

    let channel = node1.network.find_node_id(&node2.get_node_id()).unwrap();

    let vote2 = Arc::new(Vote::new_final(&key1, vec![send1.hash()]));
    node1
        .rep_crawler
        .force_process(vote2.clone(), channel.channel_id());

    assert_eq!(node1.active.confirmed(&election), false);
    // Modify online_m for online_reps to more than is available, this checks that voting below updates it to current online reps.
    node1
        .online_reps
        .lock()
        .unwrap()
        .set_online(config.online_weight_minimum + Amount::raw(20));
    node1.vote_router.vote(&vote2, VoteSource::Live);
    assert_timely(Duration::from_secs(5), || node1.active.confirmed(&election));
    assert!(node1.block(&send1.hash()).is_some());
}
