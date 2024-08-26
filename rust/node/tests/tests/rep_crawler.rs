use rsnano_core::{Amount, BlockEnum, BlockHash, KeyPair, StateBlock, Vote, DEV_GENESIS_KEY};
use rsnano_ledger::{DEV_GENESIS, DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH, DEV_GENESIS_PUB_KEY};
use rsnano_messages::{ConfirmAck, Message};
use rsnano_network::{ChannelId, ChannelMode, DropPolicy, TrafficType};
use rsnano_node::node::NodeExt;
use std::{sync::Arc, time::Duration};
use test_helpers::{assert_always_eq, assert_never, assert_timely_eq, System};

#[test]
fn ignore_rebroadcast() {
    let mut system = System::new();
    let node1 = system.make_node();
    let node2 = system.make_node();

    let channel1to2 = node1
        .network_info
        .read()
        .unwrap()
        .find_node_id(&node2.node_id.public_key())
        .expect("channel not found 1 to 2")
        .channel_id();

    let channel2to1 = node2
        .network_info
        .read()
        .unwrap()
        .find_node_id(&node1.node_id.public_key())
        .expect("channel not found 2 to 1")
        .channel_id();

    node1
        .rep_crawler
        .force_query(*DEV_GENESIS_HASH, channel1to2);

    assert_always_eq(
        Duration::from_millis(100),
        || node1.online_reps.lock().unwrap().peered_reps_count(),
        0,
    );

    // Now we spam the vote for genesis, so it appears as a rebroadcasted vote
    let vote = Vote::new(&DEV_GENESIS_KEY, 0, 0, vec![*DEV_GENESIS_HASH]);
    node1
        .rep_crawler
        .force_query(*DEV_GENESIS_HASH, channel1to2);

    let tick = || {
        let msg = Message::ConfirmAck(ConfirmAck::new_with_rebroadcasted_vote(vote.clone()));
        node2.message_publisher.lock().unwrap().try_send(
            channel2to1,
            &msg,
            DropPolicy::ShouldNotDrop,
            TrafficType::Generic,
        );
        false
    };

    assert_never(Duration::from_secs(1), || {
        tick() || node1.online_reps.lock().unwrap().peered_reps_count() > 0
    })
}

// Votes from local channels should be ignored
#[test]
fn ignore_local() {
    let mut system = System::new();
    let node = system.make_node();

    let vote = Arc::new(Vote::new(&DEV_GENESIS_KEY, 0, 0, vec![*DEV_GENESIS_HASH]));
    node.rep_crawler.force_process(vote, ChannelId::LOOPBACK);
    assert_always_eq(
        Duration::from_millis(500),
        || node.online_reps.lock().unwrap().peered_reps_count(),
        0,
    )
}

#[test]
fn rep_weight() {
    let mut system = System::new();
    let node = system.make_node();
    let node1 = system.make_node();
    let node2 = system.make_node();
    let node3 = system.make_node();
    let keypair1 = KeyPair::new();
    let keypair2 = KeyPair::new();
    let amount_pr = node.online_reps.lock().unwrap().minimum_principal_weight() + Amount::raw(100);
    let amount_not_pr =
        node.online_reps.lock().unwrap().minimum_principal_weight() - Amount::raw(100);

    let block1 = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - amount_not_pr,
        keypair1.account().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev((*DEV_GENESIS_HASH).into()),
    ));
    let block2 = BlockEnum::State(StateBlock::new(
        keypair1.account(),
        BlockHash::zero(),
        keypair1.public_key(),
        amount_not_pr,
        block1.hash().into(),
        &keypair1,
        node.work_generate_dev(keypair1.public_key().into()),
    ));
    let block3 = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        block1.hash(),
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - amount_not_pr - amount_pr,
        keypair2.account().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(block1.hash().into()),
    ));
    let block4 = BlockEnum::State(StateBlock::new(
        keypair2.account(),
        BlockHash::zero(),
        keypair2.public_key(),
        amount_pr,
        block3.hash().into(),
        &keypair2,
        node.work_generate_dev(keypair2.public_key().into()),
    ));
    let blocks = [block1, block2, block3, block4];
    node.process_multi(&blocks);
    node1.process_multi(&blocks);
    node2.process_multi(&blocks);
    node3.process_multi(&blocks);
    assert_eq!(node.online_reps.lock().unwrap().online_reps().count(), 0);

    assert_timely_eq(
        Duration::from_secs(5),
        || {
            node.network_info
                .read()
                .unwrap()
                .count_by_mode(ChannelMode::Realtime)
        },
        3,
    );

    let (channel1, channel2, channel3) = {
        let network = node.network_info.read().unwrap();
        (
            network
                .find_node_id(&node1.get_node_id())
                .unwrap()
                .channel_id(),
            network
                .find_node_id(&node2.get_node_id())
                .unwrap()
                .channel_id(),
            network
                .find_node_id(&node3.get_node_id())
                .unwrap()
                .channel_id(),
        )
    };

    let vote0 = Arc::new(Vote::new(&DEV_GENESIS_KEY, 0, 0, vec![*DEV_GENESIS_HASH]));
    let vote1 = Arc::new(Vote::new(&keypair1, 0, 0, vec![*DEV_GENESIS_HASH]));
    let vote2 = Arc::new(Vote::new(&keypair2, 0, 0, vec![*DEV_GENESIS_HASH]));

    node.rep_crawler.force_process(vote0, channel1);
    node.rep_crawler.force_process(vote1, channel2);
    node.rep_crawler.force_process(vote2, channel3);

    assert_timely_eq(
        Duration::from_secs(5),
        || node.online_reps.lock().unwrap().peered_reps_count(),
        2,
    );
    // Make sure we get the rep with the most weight first
    let rep = node.online_reps.lock().unwrap().peered_reps()[0].clone();
    assert_eq!(
        node.balance(&DEV_GENESIS_ACCOUNT),
        node.ledger.weight(&rep.account)
    );
    assert_eq!(channel1, rep.channel_id);
    assert_eq!(node.online_reps.lock().unwrap().is_pr(channel1), true);
    assert_eq!(node.online_reps.lock().unwrap().is_pr(channel2), false);
    assert_eq!(node.online_reps.lock().unwrap().is_pr(channel3), true);
}

// This test checks that if a block is in the recently_confirmed list then the repcrawler will not send a request for it.
// The behaviour of this test previously was the opposite, that the repcrawler eventually send out such a block and deleted the block
// from the recently confirmed list to try to make ammends for sending it, which is bad behaviour.
// In the long term, we should have a better way to check for reps and this test should become redundant
#[test]
fn recently_confirmed() {
    let mut system = System::new();
    let node1 = system.make_node();
    node1.active.insert_recently_confirmed(&DEV_GENESIS);

    let node2 = system.make_node();
    node2.insert_into_wallet(&DEV_GENESIS_KEY);
    let channel = node1
        .network_info
        .read()
        .unwrap()
        .find_node_id(&node2.get_node_id())
        .unwrap()
        .clone();
    node1.rep_crawler.query_channel(channel); // this query should be dropped due to the recently_confirmed entry
    assert_always_eq(
        Duration::from_millis(500),
        || node1.online_reps.lock().unwrap().peered_reps_count(),
        0,
    );
}

// Test that nodes can track nodes that have rep weight for priority broadcasting
#[test]
fn rep_list() {
    let mut system = System::new();
    let node1 = system.make_node();
    let node2 = system.make_node();
    assert_eq!(0, node2.online_reps.lock().unwrap().peered_reps_count());
    // Node #1 has a rep
    node1.insert_into_wallet(&DEV_GENESIS_KEY);
    assert_timely_eq(
        Duration::from_secs(5),
        || node2.online_reps.lock().unwrap().peered_reps_count(),
        1,
    );
    assert_eq!(
        *DEV_GENESIS_PUB_KEY,
        node2.online_reps.lock().unwrap().peered_reps()[0].account
    );
}

#[test]
fn rep_connection_close() {
    let mut system = System::new();
    let node1 = system.make_node();
    let node2 = system.make_node();
    // Add working representative (node 2)
    node2.insert_into_wallet(&DEV_GENESIS_KEY);
    assert_timely_eq(
        Duration::from_secs(10),
        || node1.online_reps.lock().unwrap().peered_reps_count(),
        1,
    );
    node2.stop();
    assert_timely_eq(
        Duration::from_secs(10),
        || node1.online_reps.lock().unwrap().peered_reps_count(),
        0,
    );
}
