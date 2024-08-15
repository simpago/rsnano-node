use rsnano_core::{
    Account, Amount, BlockEnum, BlockHash, KeyPair, StateBlock, Vote, VoteSource, DEV_GENESIS_KEY,
};
use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH};
use rsnano_node::{
    config::FrontiersConfirmationMode,
    stats::{DetailType, Direction, StatType},
    transport::ChannelId,
};
use std::{collections::HashMap, sync::Arc, time::Duration};

use super::helpers::{
    assert_timely, assert_timely_eq, assert_timely_msg, get_available_port, start_election, System,
};

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
    node.vote_processor_queue
        .vote(vote, ChannelId::from(111), VoteSource::Live);
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
    let vote = Arc::new(Vote::new(&DEV_GENESIS_KEY, 0, 0, vec![send.hash()]));
    node.vote_processor_queue
        .vote(vote, ChannelId::from(111), VoteSource::Live);
    assert_timely_eq(
        Duration::from_secs(5),
        || node.vote_cache.lock().unwrap().size(),
        1,
    );

    node.process_active(send.clone());

    assert_timely_msg(
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
    node.vote_processor_queue
        .vote(vote, ChannelId::from(111), VoteSource::Live);

    assert_timely_eq(
        Duration::from_secs(5),
        || node.vote_cache.lock().unwrap().size(),
        1,
    );

    node.process_active(send2.clone());

    assert_timely_msg(
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
    let vote1 = Arc::new(Vote::new(&key, 0, 0, vec![send.hash()]));
    node.vote_processor_queue
        .vote(vote1.clone(), ChannelId::from(111), VoteSource::Live);

    assert_timely_eq(Duration::from_secs(5), || election.vote_count(), 2);

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
    assert_eq!(election.vote_count(), 2);
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

#[test]
fn inactive_votes_cache_multiple_votes() {
    let mut system = System::new();
    let mut config = System::default_config();
    config.frontiers_confirmation = FrontiersConfirmationMode::Disabled;
    let node = system.build_node().config(config).finish();
    let key = KeyPair::new();

    let send1 = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_ACCOUNT,
        Amount::MAX - Amount::nano(100_000),
        key.public_key().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev((*DEV_GENESIS_HASH).into()),
    ));

    let send2 = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        send1.hash(),
        *DEV_GENESIS_ACCOUNT,
        Amount::nano(100_000),
        key.public_key().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(send1.hash().into()),
    ));

    let open = BlockEnum::State(StateBlock::new(
        key.public_key(),
        BlockHash::zero(),
        key.public_key(),
        Amount::nano(100_000),
        send1.hash().into(),
        &key,
        node.work_generate_dev(key.public_key().into()),
    ));

    // put the blocks in the ledger witout triggering an election
    node.process(send1.clone()).unwrap();
    node.process(send2.clone()).unwrap();
    node.process(open.clone()).unwrap();

    // Process votes
    let vote1 = Arc::new(Vote::new(&key, 0, 0, vec![send1.hash()]));
    node.vote_processor_queue
        .vote(vote1, ChannelId::from(111), VoteSource::Live);

    let vote2 = Arc::new(Vote::new(&DEV_GENESIS_KEY, 0, 0, vec![send1.hash()]));
    node.vote_processor_queue
        .vote(vote2, ChannelId::from(222), VoteSource::Live);

    assert_timely_eq(
        Duration::from_secs(5),
        || node.vote_cache.lock().unwrap().find(&send1.hash()).len(),
        2,
    );
    assert_eq!(1, node.vote_cache.lock().unwrap().size());
    let election = start_election(&node, &send1.hash());
    assert_timely_eq(Duration::from_secs(5), || election.vote_count(), 3); // 2 votes and 1 default not_an_account
    assert_eq!(
        2,
        node.stats
            .count(StatType::ElectionVote, DetailType::Cache, Direction::In)
    );
}

#[test]
fn inactive_votes_cache_election_start() {
    let mut system = System::new();
    let mut config = System::default_config();
    config.frontiers_confirmation = FrontiersConfirmationMode::Disabled;
    config.optimistic_scheduler.enabled = false;
    config.priority_scheduler_enabled = false;
    let node = system.build_node().config(config).finish();
    let key1 = KeyPair::new();
    let key2 = KeyPair::new();

    // Enough weight to trigger election hinting but not enough to confirm block on its own
    let amount = ((node
        .online_reps
        .lock()
        .unwrap()
        .trended_weight_or_minimum_online_weight()
        / 100)
        * node.config.hinted_scheduler.hinting_threshold_percent as u128)
        / 2
        + Amount::nano(1_000_000);

    let send1 = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_ACCOUNT,
        Amount::MAX - amount,
        key1.public_key().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev((*DEV_GENESIS_HASH).into()),
    ));
    let send2 = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        send1.hash(),
        *DEV_GENESIS_ACCOUNT,
        Amount::MAX - (amount * 2),
        key2.public_key().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(send1.hash().into()),
    ));
    let open1 = BlockEnum::State(StateBlock::new(
        key1.public_key(),
        BlockHash::zero(),
        key1.public_key(),
        amount,
        send1.hash().into(),
        &key1,
        node.work_generate_dev(key1.public_key().into()),
    ));
    let open2 = BlockEnum::State(StateBlock::new(
        key2.public_key(),
        BlockHash::zero(),
        key2.public_key(),
        amount,
        send2.hash().into(),
        &key2,
        node.work_generate_dev(key2.public_key().into()),
    ));
    node.process(send1.clone()).unwrap();
    node.process(send2.clone()).unwrap();
    node.process(open1.clone()).unwrap();
    node.process(open2.clone()).unwrap();

    // These blocks will be processed later
    let send3 = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        send2.hash(),
        *DEV_GENESIS_ACCOUNT,
        send2.balance() - Amount::raw(1),
        Account::from(2).into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(send2.hash().into()),
    ));
    let send4 = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        send3.hash(),
        *DEV_GENESIS_ACCOUNT,
        send3.balance() - Amount::raw(1),
        Account::from(3).into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(send3.hash().into()),
    ));

    // Inactive votes
    let vote1 = Arc::new(Vote::new(
        &key1,
        0,
        0,
        vec![open1.hash(), open2.hash(), send4.hash()],
    ));
    let channel = ChannelId::from(111);
    node.vote_processor_queue
        .vote(vote1, channel, VoteSource::Live);
    assert_timely_eq(
        Duration::from_secs(5),
        || node.vote_cache.lock().unwrap().size(),
        3,
    );
    assert_eq!(node.active.len(), 0);
    assert_eq!(1, node.ledger.cemented_count());

    // 2 votes are required to start election (dev network)
    let vote2 = Arc::new(Vote::new(
        &key2,
        0,
        0,
        vec![open1.hash(), open2.hash(), send4.hash()],
    ));
    node.vote_processor_queue
        .vote(vote2, channel, VoteSource::Live);
    // Only election for send1 should start, other blocks are missing dependencies and don't have enough final weight
    assert_timely_eq(Duration::from_secs(5), || node.active.len(), 1);
    assert!(node.vote_router.active(&send1.hash()));

    // Confirm elections with weight quorum
    let vote0 = Arc::new(Vote::new_final(
        &DEV_GENESIS_KEY,
        vec![open1.hash(), open2.hash(), send4.hash()],
    ));
    node.vote_processor_queue
        .vote(vote0, channel, VoteSource::Live);
    assert_timely_eq(Duration::from_secs(5), || node.active.len(), 0);
    assert_timely_eq(Duration::from_secs(5), || node.ledger.cemented_count(), 5);
    assert!(node.blocks_confirmed(&[send1, send2, open1, open2]));

    // A late block arrival also checks the inactive votes cache
    assert_eq!(node.active.len(), 0);
    let send4_cache = node.vote_cache.lock().unwrap().find(&send4.hash());
    assert_eq!(3, send4_cache.len());
    node.process_active(send3.clone());
    // An election is started for send6 but does not
    let tx = node.ledger.read_txn();
    assert_eq!(
        node.ledger.confirmed().block_exists(&tx, &send3.hash()),
        false
    );
    assert_eq!(node.confirming_set.exists(&send3.hash()), false);
    // send7 cannot be voted on but an election should be started from inactive votes
    assert_eq!(node.ledger.dependents_confirmed(&tx, &send4), false);
    node.process_active(send4);
    assert_timely_eq(Duration::from_secs(5), || node.ledger.cemented_count(), 7);
}

#[test]
fn republish_winner() {
    let mut system = System::new();
    let mut config = System::default_config();
    config.frontiers_confirmation = FrontiersConfirmationMode::Disabled;
    let node1 = system.build_node().config(config.clone()).finish();
    config.peering_port = Some(get_available_port());
    let node2 = system.build_node().config(config).finish();

    let key = KeyPair::new();
    let send1 = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_ACCOUNT,
        Amount::MAX - Amount::nano(1000),
        key.public_key().into(),
        &DEV_GENESIS_KEY,
        node1.work_generate_dev((*DEV_GENESIS_HASH).into()),
    ));

    node1.process_active(send1.clone());
    assert_timely(Duration::from_secs(5), || node1.block_exists(&send1.hash()));

    assert_timely_eq(
        Duration::from_secs(3),
        || {
            node2
                .stats
                .count(StatType::Message, DetailType::Publish, Direction::In)
        },
        1,
    );

    // Several forks
    for i in 0..5 {
        let fork = BlockEnum::State(StateBlock::new(
            *DEV_GENESIS_ACCOUNT,
            *DEV_GENESIS_HASH,
            *DEV_GENESIS_ACCOUNT,
            Amount::MAX - Amount::raw(1 + i),
            key.public_key().into(),
            &DEV_GENESIS_KEY,
            node1.work_generate_dev((*DEV_GENESIS_HASH).into()),
        ));
        node1.process_active(fork.clone());
        assert_timely(Duration::from_secs(5), || node1.active.active(&fork));
    }

    assert_timely(Duration::from_secs(3), || node1.active.len() > 0);
    assert_eq!(
        1,
        node2
            .stats
            .count(StatType::Message, DetailType::Publish, Direction::In)
    );

    // Process new fork with vote to change winner
    let fork = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_ACCOUNT,
        Amount::MAX - Amount::nano(2000),
        key.public_key().into(),
        &DEV_GENESIS_KEY,
        node1.work_generate_dev((*DEV_GENESIS_HASH).into()),
    ));
    node1.process_active(fork.clone());
    assert_timely(Duration::from_secs(5), || {
        node1.vote_router.active(&fork.hash())
    });

    let election = node1.active.election(&fork.qualified_root()).unwrap();
    let vote = Arc::new(Vote::new_final(&DEV_GENESIS_KEY, vec![fork.hash()]));
    node1
        .vote_processor_queue
        .vote(vote, ChannelId::from(111), VoteSource::Live);
    assert_timely(Duration::from_secs(5), || node1.active.confirmed(&election));

    assert_eq!(fork.hash(), election.winner_hash().unwrap());

    assert_timely(Duration::from_secs(5), || {
        node2.block_confirmed(&fork.hash())
    });
}
