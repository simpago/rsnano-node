use rsnano_core::{
    Amount, BlockEnum, KeyPair, Signature, StateBlock, Vote, VoteCode, VoteSource, DEV_GENESIS_KEY,
};
use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH, DEV_GENESIS_PUB_KEY};
use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};
use test_helpers::{assert_timely, make_fake_channel, start_election, System};

#[test]
fn check_signature() {
    let mut system = System::new();
    let mut config = System::default_config();
    config.online_weight_minimum = Amount::MAX;
    let node = system.build_node().config(config).finish();
    let key1 = KeyPair::new();
    let send1 = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::raw(100),
        key1.account().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev((*DEV_GENESIS_HASH).into()),
    ));
    node.process(send1.clone()).unwrap();
    let election1 = start_election(&node, &send1.hash());
    assert_eq!(1, election1.vote_count());
    let mut vote1 = Vote::new(&DEV_GENESIS_KEY, Vote::TIMESTAMP_MIN, 0, vec![send1.hash()]);
    let good_signature = vote1.signature;
    vote1.signature = Signature::new();
    let channel = make_fake_channel(&node);
    assert_eq!(
        VoteCode::Invalid,
        node.vote_processor.vote_blocking(
            &Arc::new(vote1.clone()),
            channel.channel_id(),
            VoteSource::Live
        )
    );

    vote1.signature = good_signature;
    assert_eq!(
        VoteCode::Vote,
        node.vote_processor.vote_blocking(
            &Arc::new(vote1.clone()),
            channel.channel_id(),
            VoteSource::Live
        )
    );
    assert_eq!(
        VoteCode::Replay,
        node.vote_processor.vote_blocking(
            &Arc::new(vote1.clone()),
            channel.channel_id(),
            VoteSource::Live
        )
    );
}

// Lower timestamps are ignored
#[test]
fn add_old() {
    let mut system = System::new();
    let node = system.make_node();
    let key1 = KeyPair::new();
    let send1 = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::zero(),
        key1.account().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev((*DEV_GENESIS_HASH).into()),
    ));
    node.process(send1.clone()).unwrap();
    start_election(&node, &send1.hash());
    assert_timely(Duration::from_secs(5), || {
        node.active.election(&send1.qualified_root()).is_some()
    });
    let election1 = node.active.election(&send1.qualified_root()).unwrap();
    let vote1 = Arc::new(Vote::new(
        &DEV_GENESIS_KEY,
        Vote::TIMESTAMP_MIN * 2,
        0,
        vec![send1.hash()],
    ));
    let channel = make_fake_channel(&node);
    node.vote_processor
        .vote_blocking(&vote1, channel.channel_id(), VoteSource::Live);

    let key2 = KeyPair::new();
    let send2 = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::zero(),
        key2.account().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev((*DEV_GENESIS_HASH).into()),
    ));
    let vote2 = Arc::new(Vote::new(
        &DEV_GENESIS_KEY,
        Vote::TIMESTAMP_MIN * 1,
        0,
        vec![send2.hash()],
    ));

    election1
        .mutex
        .lock()
        .unwrap()
        .last_votes
        .get_mut(&DEV_GENESIS_PUB_KEY)
        .unwrap()
        .time = SystemTime::now() - Duration::from_secs(20);
    node.vote_processor
        .vote_blocking(&vote2, channel.channel_id(), VoteSource::Live);
    assert_eq!(2, election1.vote_count());
    let votes = election1.mutex.lock().unwrap().last_votes.clone();
    assert!(votes.contains_key(&DEV_GENESIS_PUB_KEY));
    assert_eq!(send1.hash(), votes.get(&DEV_GENESIS_PUB_KEY).unwrap().hash);
    assert_eq!(send1.hash(), election1.winner_hash().unwrap());
}

// The voting cooldown is respected
#[test]
fn add_cooldown() {
    let mut system = System::new();
    let node = system.make_node();
    let key1 = KeyPair::new();
    let send1 = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::zero(),
        key1.account().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev((*DEV_GENESIS_HASH).into()),
    ));
    node.process(send1.clone()).unwrap();
    start_election(&node, &send1.hash());
    assert_timely(Duration::from_secs(5), || {
        node.active.election(&send1.qualified_root()).is_some()
    });
    let election1 = node.active.election(&send1.qualified_root()).unwrap();
    let vote1 = Arc::new(Vote::new(
        &DEV_GENESIS_KEY,
        Vote::TIMESTAMP_MIN * 1,
        0,
        vec![send1.hash()],
    ));
    let channel = make_fake_channel(&node);
    node.vote_processor
        .vote_blocking(&vote1, channel.channel_id(), VoteSource::Live);

    let key2 = KeyPair::new();
    let send2 = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::zero(),
        key2.account().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev((*DEV_GENESIS_HASH).into()),
    ));
    let vote2 = Arc::new(Vote::new(
        &DEV_GENESIS_KEY,
        Vote::TIMESTAMP_MIN * 2,
        0,
        vec![send2.hash()],
    ));

    node.vote_processor
        .vote_blocking(&vote2, channel.channel_id(), VoteSource::Live);
    assert_eq!(2, election1.vote_count());
    let votes = election1.mutex.lock().unwrap().last_votes.clone();
    assert!(votes.contains_key(&DEV_GENESIS_PUB_KEY));
    assert_eq!(send1.hash(), votes.get(&DEV_GENESIS_PUB_KEY).unwrap().hash);
    assert_eq!(send1.hash(), election1.winner_hash().unwrap());
}
