use std::time::Duration;

use super::helpers::{assert_always_eq, assert_never, System};
use rsnano_core::{Vote, DEV_GENESIS_KEY};
use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH};
use rsnano_messages::{ConfirmAck, Message};
use rsnano_node::transport::{BufferDropPolicy, TrafficType};

#[test]
fn ignore_rebroadcast() {
    let mut system = System::new();
    let node1 = system.make_node();
    let node2 = system.make_node();

    let channel1to2 = node1
        .network
        .find_node_id(&node2.node_id.public_key())
        .expect("channel not found 1 to 2");

    let channel2to1 = node2
        .network
        .find_node_id(&node1.node_id.public_key())
        .expect("channel not found 2 to 1");

    node1
        .rep_crawler
        .force_query(*DEV_GENESIS_HASH, channel1to2.clone());

    assert_always_eq(
        Duration::from_millis(100),
        || node1.online_reps.lock().unwrap().peered_reps_count(),
        0,
    );

    // Now we spam the vote for genesis, so it appears as a rebroadcasted vote
    let vote = Vote::new(
        *DEV_GENESIS_ACCOUNT,
        &DEV_GENESIS_KEY.private_key(),
        0,
        0,
        vec![*DEV_GENESIS_HASH],
    );
    node1
        .rep_crawler
        .force_query(*DEV_GENESIS_HASH, channel1to2);

    let tick = || {
        let msg = Message::ConfirmAck(ConfirmAck::new_with_rebroadcasted_vote(vote.clone()));
        channel2to1.try_send(&msg, BufferDropPolicy::NoSocketDrop, TrafficType::Generic);
        false
    };

    assert_never(Duration::from_secs(1), || {
        tick() || node1.online_reps.lock().unwrap().peered_reps_count() > 0
    })
}
