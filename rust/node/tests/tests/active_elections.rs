use rsnano_core::{
    work::WorkPool, Account, Amount, BlockEnum, BlockHash, KeyPair, StateBlock, Vote, VoteSource,
    DEV_GENESIS_KEY,
};
use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH, DEV_GENESIS_PUB_KEY};
use rsnano_node::{
    config::{FrontiersConfirmationMode, NodeFlags},
    stats::{DetailType, Direction, StatType},
    transport::ChannelId,
    wallets::WalletsExt,
};
use std::{
    collections::HashMap,
    sync::{atomic::Ordering, Arc},
    thread::sleep,
    time::Duration,
};

use test_helpers::{
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
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::raw(100),
        key.account().into(),
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
        *DEV_GENESIS_PUB_KEY,
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
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::raw(100),
        key.account().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev((*DEV_GENESIS_HASH).into()),
    ));

    let send2 = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::raw(200),
        key.account().into(),
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
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - rep_weight,
        key.account().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev((*DEV_GENESIS_HASH).into()),
    ));

    let open = BlockEnum::State(StateBlock::new(
        key.account(),
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
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::nano(100_000),
        key.account().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev((*DEV_GENESIS_HASH).into()),
    ));

    let send2 = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        send1.hash(),
        *DEV_GENESIS_PUB_KEY,
        Amount::nano(100_000),
        key.account().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(send1.hash().into()),
    ));

    let open = BlockEnum::State(StateBlock::new(
        key.account(),
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
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - amount,
        key1.account().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev((*DEV_GENESIS_HASH).into()),
    ));
    let send2 = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        send1.hash(),
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - (amount * 2),
        key2.account().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(send1.hash().into()),
    ));
    let open1 = BlockEnum::State(StateBlock::new(
        key1.account(),
        BlockHash::zero(),
        key1.public_key(),
        amount,
        send1.hash().into(),
        &key1,
        node.work_generate_dev(key1.public_key().into()),
    ));
    let open2 = BlockEnum::State(StateBlock::new(
        key2.account(),
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
        *DEV_GENESIS_PUB_KEY,
        send2.balance() - Amount::raw(1),
        Account::from(2).into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(send2.hash().into()),
    ));
    let send4 = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        send3.hash(),
        *DEV_GENESIS_PUB_KEY,
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
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::nano(1000),
        key.account().into(),
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
            *DEV_GENESIS_PUB_KEY,
            Amount::MAX - Amount::raw(1 + i),
            key.account().into(),
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
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::nano(2000),
        key.account().into(),
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

/*
 * Tests that an election can be confirmed as the result of a confirmation request
 *
 * Set-up:
 * - node1 with:
 * 		- enabled frontiers_confirmation (default) -> allows it to confirm blocks and subsequently generates votes
 * - node2 with:
 * 		- disabled rep crawler -> this inhibits node2 from learning that node1 is a rep
 */
#[test]
fn confirm_election_by_request() {
    let mut system = System::new();
    let node1 = system.make_node();

    let send1 = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::raw(100),
        1.into(),
        &DEV_GENESIS_KEY,
        node1.work_generate_dev((*DEV_GENESIS_HASH).into()),
    ));

    // Process send1 locally on node1
    node1.process(send1.clone()).unwrap();

    // Add rep key to node1
    let wallet_id = node1.wallets.wallet_ids()[0];
    node1
        .wallets
        .insert_adhoc2(&wallet_id, &DEV_GENESIS_KEY.private_key(), true)
        .unwrap();

    // Ensure election on node1 is already confirmed before connecting with node2
    assert_timely(Duration::from_secs(5), || {
        node1.block_confirmed(&send1.hash())
    });

    // Wait for the election to be removed and give time for any in-flight vote broadcasts to settle
    assert_timely(Duration::from_secs(5), || node1.active.len() == 0);
    sleep(Duration::from_secs(1));

    // At this point node1 should not generate votes for send1 block unless it receives a request

    // Create a second node
    let flags = NodeFlags {
        disable_rep_crawler: true,
        ..Default::default()
    };
    let node2 = system.build_node().flags(flags).finish();

    // Process send1 block as live block on node2, this should start an election
    node2.process_active(send1.clone());

    // Ensure election is started on node2
    assert_timely(Duration::from_secs(5), || {
        node2.active.election(&send1.qualified_root()).is_some()
    });

    let election = node2.active.election(&send1.qualified_root()).unwrap();

    // Ensure election on node2 did not get confirmed without us requesting votes
    sleep(Duration::from_secs(1));
    assert_eq!(node2.active.confirmed(&election), false);

    // Expect that node2 has nobody to send a confirmation_request to (no reps)
    assert_eq!(
        election.confirmation_request_count.load(Ordering::SeqCst),
        0
    );

    // Get random peer list (of size 1) from node2 -- so basically just node2
    let peers = node2
        .network_info
        .read()
        .unwrap()
        .random_realtime_channels(1, 0);
    assert_eq!(peers.is_empty(), false);

    // Add representative (node1) to disabled rep crawler of node2
    node2.online_reps.lock().unwrap().vote_observed_directly(
        *DEV_GENESIS_PUB_KEY,
        peers[0].channel_id(),
        node2.steady_clock.now(),
    );

    // Expect a vote to come back
    assert_timely(Duration::from_secs(5), || election.vote_count() >= 1);

    // There needs to be at least one request to get the election confirmed,
    // Rep has this block already confirmed so should reply with final vote only
    assert_timely(Duration::from_secs(5), || {
        election.confirmation_request_count.load(Ordering::SeqCst) >= 1
    });

    // Expect election was confirmed
    assert_timely(Duration::from_secs(5), || node2.active.confirmed(&election));
    assert_timely(Duration::from_secs(5), || {
        node1.block_confirmed(&send1.hash())
    });
    assert_timely(Duration::from_secs(5), || {
        node2.block_confirmed(&send1.hash())
    });
}

#[test]
fn confirm_frontier() {
    let mut system = System::new();

    // send 100 raw from genesis to a random account
    let send = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::raw(100),
        1.into(),
        &DEV_GENESIS_KEY,
        system
            .work
            .generate_dev2((*DEV_GENESIS_HASH).into())
            .unwrap(),
    ));

    // Voting node
    let flags = NodeFlags {
        disable_request_loop: true,
        disable_ongoing_bootstrap: true,
        disable_ascending_bootstrap: true,
        ..Default::default()
    };
    let node1 = system.build_node().flags(flags).finish();
    let wallet_id = node1.wallets.wallet_ids()[0];
    node1
        .wallets
        .insert_adhoc2(&wallet_id, &DEV_GENESIS_KEY.private_key(), true)
        .unwrap();

    node1.process(send.clone()).unwrap();
    node1.confirm(send.hash());

    // The rep crawler would otherwise request confirmations in order to find representatives
    let flags2 = NodeFlags {
        disable_ongoing_bootstrap: true,
        disable_ascending_bootstrap: true,
        disable_rep_crawler: true,
        ..Default::default()
    };
    // start node2 later so that we do not get the gossip traffic
    let node2 = system.build_node().flags(flags2).finish();

    // Add representative to disabled rep crawler
    let peers = node2
        .network_info
        .read()
        .unwrap()
        .random_realtime_channels(1, 0);
    assert!(!peers.is_empty());
    node2.online_reps.lock().unwrap().vote_observed_directly(
        *DEV_GENESIS_PUB_KEY,
        peers[0].channel_id(),
        node2.steady_clock.now(),
    );

    node2.process(send.clone()).unwrap();
    assert_timely(Duration::from_secs(5), || node2.active.len() > 0);

    // Save election to check request count afterwards
    assert_timely(Duration::from_secs(5), || {
        node2.active.election(&send.qualified_root()).is_some()
    });
    let election2 = node2.active.election(&send.qualified_root()).unwrap();

    assert_timely(Duration::from_secs(5), || {
        node2.block_confirmed(&send.hash())
    });
    assert_timely_eq(Duration::from_secs(5), || node2.ledger.cemented_count(), 2);
    assert_timely(Duration::from_secs(5), || node2.active.len() == 0);
    assert!(election2.confirmation_request_count.load(Ordering::SeqCst) > 0);
}
