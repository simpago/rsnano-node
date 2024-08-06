//use rsnano_core::{
//    work::WorkPool, Amount, BlockEnum, BlockHash, KeyPair, StateBlock, Vote, DEV_GENESIS_KEY,
//};
//use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH};
//use rsnano_node::config::FrontiersConfirmationMode;
//
//use super::helpers::System;

use std::{collections::HashMap, sync::Arc, time::Duration};

use futures_util::sink::drain;
use rsnano_core::{
    Account, Amount, BlockEnum, BlockHash, KeyPair, StateBlock, Vote, VoteSource, DEV_GENESIS_KEY,
};
use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH};
use rsnano_node::{
    config::FrontiersConfirmationMode,
    stats::{DetailType, Direction, StatType},
};

use super::helpers::{assert_timely, assert_timely_eq, make_fake_channel, start_election, System};

/// What this test is doing:
/// Create 20 representatives with minimum principal weight each
/// Create a send block on the genesis account (the last send block)
/// Create 20 forks of the last send block using genesis as representative (no votes produced)
/// Check that only 10 blocks remain in the election (due to max 10 forks per election object limit)
/// Create 20 more forks of the last send block using the new reps as representatives and produce votes for them
///     (9 votes from this batch should survive and replace existing blocks in the election, why not 10?)
/// Then send winning block and it should replace one of the existing blocks
#[test]
fn fork_replacement_tally() {
    //let mut system = System::new();
    //let mut node_config = System::default_config();
    //node_config.frontiers_confirmation = FrontiersConfirmationMode::Disabled;
    //let node1 = system.build_node().config(node_config).finish();

    //const REPS_COUNT: usize = 20;
    //const MAX_BLOCKS: usize = 10;

    //let keys: Vec<_> = std::iter::repeat_with(|| KeyPair::new())
    //    .take(REPS_COUNT)
    //    .collect();
    //let mut latest = *DEV_GENESIS_HASH;
    //let mut balance = Amount::MAX;
    //let amount = node1.online_reps.lock().unwrap().minimum_principal_weight();

    //// Create 20 representatives & confirm blocks
    //for i in 0..REPS_COUNT {
    //    balance = balance - (amount + Amount::raw(i as u128));
    //    let send = BlockEnum::State(StateBlock::new(
    //        *DEV_GENESIS_ACCOUNT,
    //        latest,
    //        *DEV_GENESIS_ACCOUNT,
    //        balance,
    //        keys[i].public_key().into(),
    //        &DEV_GENESIS_KEY,
    //        system.work.generate_dev2(latest.into()).unwrap(),
    //    ));
    //    node1.process_active(send.clone());
    //    latest = send.hash();
    //    let open = BlockEnum::State(StateBlock::new(
    //        keys[i].public_key(),
    //        BlockHash::zero(),
    //        keys[i].public_key(),
    //        amount + Amount::raw(i as u128),
    //        send.hash().into(),
    //        &keys[i],
    //        system
    //            .work
    //            .generate_dev2(keys[i].public_key().into())
    //            .unwrap(),
    //    ));
    //    node1.process_active(open.clone());
    //    // Confirmation
    //    let vote = Vote::new_final(&DEV_GENESIS_KEY, vec![send.hash(), open.hash()]);
    //    node1
    //        .vote_processor_queue
    //        .vote(Arc::new(vote), channel, source)
    //}
    // TODO port remainig part
}

#[test]
fn inactive_votes_cache_basic() {
    let mut system = System::new();
    let node = system.make_node();
    let key = KeyPair::new();
    let send = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_ACCOUNT,
        Amount::MAX - Amount::raw(100),
        key.public_key().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev((*DEV_GENESIS_HASH).into()),
    ));
    let vote = Arc::new(Vote::new_final(&DEV_GENESIS_KEY, vec![send.hash()]));
    let channel = make_fake_channel(&node);
    node.vote_processor_queue
        .vote(vote, &channel, VoteSource::Live);
    assert_timely_eq(
        Duration::from_secs(5),
        || node.vote_cache.lock().unwrap().size(),
        1,
    );
    node.process_active(send.clone());
    assert_timely_eq(
        Duration::from_secs(5),
        || node.block_confirmed(&send.hash()),
        true,
    );
    assert_eq!(
        1,
        node.stats
            .count(StatType::ElectionVote, DetailType::Cache, Direction::In)
    )
}

// This test case confirms that a non final vote cannot cause an election to become confirmed
#[test]
fn non_final() {
    let mut system = System::new();
    let node = system.make_node();

    let send = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_ACCOUNT,
        Amount::MAX - Amount::raw(100),
        Account::from(42).into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev((*DEV_GENESIS_HASH).into()),
    ));

    // Non-final vote
    let vote = Arc::new(Vote::new(
        *DEV_GENESIS_ACCOUNT,
        &DEV_GENESIS_KEY.private_key(),
        0,
        0,
        vec![send.hash()],
    ));
    let channel = make_fake_channel(&node);
    node.vote_processor_queue
        .vote(vote, &channel, VoteSource::Live);
    assert_timely_eq(
        Duration::from_secs(5),
        || node.vote_cache.lock().unwrap().size(),
        1,
    );

    node.process_active(send.clone());

    assert_timely(
        Duration::from_secs(5),
        || node.active.election(&send.qualified_root()).is_some(),
        "election not found",
    );

    let election = node.active.election(&send.qualified_root()).unwrap();
    assert_timely_eq(
        Duration::from_secs(5),
        || {
            node.stats
                .count(StatType::ElectionVote, DetailType::Cache, Direction::In)
        },
        1,
    );

    assert_timely_eq(
        Duration::from_secs(5),
        || {
            node.active
                .vote_applier
                .tally_impl(&mut election.mutex.lock().unwrap())
                .first_key_value()
                .unwrap()
                .0
                 .0
        },
        Amount::MAX - Amount::raw(100),
    );
    assert_eq!(node.active.confirmed(&election), false);
}

#[test]
fn inactive_votes_cache_fork() {
    let mut system = System::new();
    let node = system.make_node();
    let key = KeyPair::new();

    let send1 = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_ACCOUNT,
        Amount::MAX - Amount::raw(100),
        key.public_key().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev((*DEV_GENESIS_HASH).into()),
    ));

    let send2 = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_ACCOUNT,
        Amount::MAX - Amount::raw(200),
        key.public_key().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev((*DEV_GENESIS_HASH).into()),
    ));

    let vote = Arc::new(Vote::new_final(&DEV_GENESIS_KEY, vec![send1.hash()]));
    let channel = make_fake_channel(&node);
    node.vote_processor_queue
        .vote(vote, &channel, VoteSource::Live);

    assert_timely_eq(
        Duration::from_secs(5),
        || node.vote_cache.lock().unwrap().size(),
        1,
    );

    node.process_active(send2.clone());

    assert_timely(
        Duration::from_secs(5),
        || node.active.election(&send1.qualified_root()).is_some(),
        "election not found",
    );

    let election = node.active.election(&send1.qualified_root()).unwrap();

    node.process_active(send1.clone());

    assert_timely_eq(
        Duration::from_secs(5),
        || election.mutex.lock().unwrap().last_blocks.len(),
        2,
    );

    assert_timely_eq(
        Duration::from_secs(5),
        || node.block_confirmed(&send1.hash()),
        true,
    );
    assert_eq!(
        1,
        node.stats
            .count(StatType::ElectionVote, DetailType::Cache, Direction::In)
    )
}

#[test]
fn inactive_votes_cache_existing_vote() {
    let mut system = System::new();
    let mut config = System::default_config();
    config.frontiers_confirmation = FrontiersConfirmationMode::Disabled;
    let node = system.build_node().config(config).finish();
    let key = KeyPair::new();
    let rep_weight = Amount::nano(100_000);

    let send = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_ACCOUNT,
        Amount::MAX - rep_weight,
        key.public_key().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev((*DEV_GENESIS_HASH).into()),
    ));

    let open = BlockEnum::State(StateBlock::new(
        key.public_key(),
        BlockHash::zero(),
        key.public_key(),
        rep_weight,
        send.hash().into(),
        &key,
        node.work_generate_dev(key.public_key().into()),
    ));

    node.process(send.clone()).unwrap();
    node.process(open.clone()).unwrap();

    let election = start_election(&node, &send.hash());
    assert!(
        node.ledger.weight(&key.public_key())
            > node.online_reps.lock().unwrap().minimum_principal_weight()
    );

    // Insert vote
    let vote1 = Arc::new(Vote::new(
        key.public_key(),
        &key.private_key(),
        0,
        0,
        vec![send.hash()],
    ));
    let channel = make_fake_channel(&node);
    node.vote_processor_queue
        .vote(vote1.clone(), &channel, VoteSource::Live);

    assert_timely_eq(
        Duration::from_secs(5),
        || election.mutex.lock().unwrap().last_votes.len(),
        2,
    );

    assert_eq!(
        1,
        node.stats
            .count(StatType::Election, DetailType::Vote, Direction::In)
    );

    let last_vote1 = election
        .mutex
        .lock()
        .unwrap()
        .last_votes
        .get(&key.public_key())
        .unwrap()
        .clone();

    assert_eq!(send.hash(), last_vote1.hash);

    // Attempt to change vote with inactive_votes_cache
    node.vote_cache
        .lock()
        .unwrap()
        .insert(&vote1, rep_weight, &HashMap::new());

    let cached = node.vote_cache.lock().unwrap().find(&send.hash());
    assert_eq!(cached.len(), 1);
    node.vote_router.vote(&cached[0], VoteSource::Live);

    // Check that election data is not changed
    assert_eq!(election.mutex.lock().unwrap().last_votes.len(), 2,);
    let last_vote2 = election
        .mutex
        .lock()
        .unwrap()
        .last_votes
        .get(&key.public_key())
        .unwrap()
        .clone();
    assert_eq!(send.hash(), last_vote2.hash);
    assert_eq!(
        0,
        node.stats
            .count(StatType::ElectionVote, DetailType::Cache, Direction::In)
    );
}
