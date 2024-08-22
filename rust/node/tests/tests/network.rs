use rsnano_core::{Account, Amount, BlockEnum, KeyPair, StateBlock, Vote, DEV_GENESIS_KEY};
use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH, DEV_GENESIS_PUB_KEY};
use rsnano_messages::{ConfirmAck, Keepalive, Message, Publish};
use rsnano_node::{
    stats::{DetailType, Direction, StatType},
    transport::{ChannelMode, DropPolicy, TrafficType},
};
use std::{ops::Deref, sync::Arc, time::Duration};
use test_helpers::{
    assert_timely_eq, assert_timely_msg, establish_tcp, make_fake_channel, start_election, System,
};

#[test]
fn last_contacted() {
    let mut system = System::new();

    let node0 = system.make_node();

    let mut node1_config = System::default_config();
    node1_config.tcp_incoming_connections_max = 0; // Prevent ephemeral node1->node0 channel repacement with incoming connection
    let node1 = system
        .build_node()
        .config(node1_config)
        .disconnected()
        .finish();

    let channel1 = establish_tcp(&node1, &node0);
    assert_timely_eq(
        Duration::from_secs(3),
        || {
            node0
                .network_info
                .read()
                .unwrap()
                .count_by_mode(ChannelMode::Realtime)
        },
        1,
    );

    // channel0 is the other side of channel1, same connection different endpoint
    let channel0 = node0
        .network_info
        .read()
        .unwrap()
        .find_node_id(&node1.node_id.public_key())
        .unwrap()
        .clone();

    // check that the endpoints are part of the same connection
    assert_eq!(channel0.local_addr(), channel1.peer_addr());
    assert_eq!(channel1.local_addr(), channel0.peer_addr());

    // capture the state before and ensure the clock ticks at least once
    let timestamp_before_keepalive = channel0.last_packet_received();
    let keepalive_count =
        node0
            .stats
            .count(StatType::Message, DetailType::Keepalive, Direction::In);
    assert_timely_msg(
        Duration::from_secs(3),
        || node0.steady_clock.now() > timestamp_before_keepalive,
        "clock did not advance",
    );

    // send 3 keepalives
    // we need an extra keepalive to handle the race condition between the timestamp set and the counter increment
    // and we need one more keepalive to handle the possibility that there is a keepalive already in flight when we start the crucial part of the test
    // it is possible that there could be multiple keepalives in flight but we assume here that there will be no more than one in flight for the purposes of this test
    let keepalive = Message::Keepalive(Keepalive::default());
    let mut publisher = node0.message_publisher.lock().unwrap();
    publisher.try_send(
        channel1.channel_id(),
        &keepalive,
        DropPolicy::ShouldNotDrop,
        TrafficType::Generic,
    );
    publisher.try_send(
        channel1.channel_id(),
        &keepalive,
        DropPolicy::ShouldNotDrop,
        TrafficType::Generic,
    );
    publisher.try_send(
        channel1.channel_id(),
        &keepalive,
        DropPolicy::ShouldNotDrop,
        TrafficType::Generic,
    );

    assert_timely_msg(
        Duration::from_secs(3),
        || {
            node0
                .stats
                .count(StatType::Message, DetailType::Keepalive, Direction::In)
                >= keepalive_count + 3
        },
        "keepalive count",
    );
    assert_eq!(
        node0
            .network_info
            .read()
            .unwrap()
            .count_by_mode(ChannelMode::Realtime),
        1
    );
    let timestamp_after_keepalive = channel0.last_packet_received();
    assert!(timestamp_after_keepalive > timestamp_before_keepalive);
}

#[test]
fn send_discarded_publish() {
    let mut system = System::new();
    let node1 = system.make_node();
    let node2 = system.make_node();

    let block = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        2.into(),
        3.into(),
        Amount::MAX,
        4.into(),
        &DEV_GENESIS_KEY,
        node1.work_generate_dev(2.into()),
    ));

    node1.message_publisher.lock().unwrap().flood(
        &Message::Publish(Publish::new_forward(block)),
        DropPolicy::ShouldNotDrop,
        1.0,
    );

    assert_eq!(node1.latest(&DEV_GENESIS_ACCOUNT), *DEV_GENESIS_HASH);
    assert_eq!(node2.latest(&DEV_GENESIS_ACCOUNT), *DEV_GENESIS_HASH);
    assert_timely_msg(
        Duration::from_secs(10),
        || {
            node2
                .stats
                .count(StatType::Message, DetailType::Publish, Direction::In)
                != 0
        },
        "no publish received",
    );
    assert_eq!(node1.latest(&DEV_GENESIS_ACCOUNT), *DEV_GENESIS_HASH);
    assert_eq!(node2.latest(&DEV_GENESIS_ACCOUNT), *DEV_GENESIS_HASH);
}

#[test]
fn receivable_processor_confirm_insufficient_pos() {
    let mut system = System::new();
    let node1 = system.make_node();
    let send1 = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::raw(1),
        Account::zero().into(),
        &DEV_GENESIS_KEY,
        node1.work_generate_dev((*DEV_GENESIS_HASH).into()),
    ));

    node1.process(send1.clone()).unwrap();
    let election = start_election(&node1, &send1.hash());

    let key1 = KeyPair::new();
    let vote = Arc::new(Vote::new_final(&key1, vec![send1.hash()]));
    let channel = make_fake_channel(&node1);
    let con1 = Message::ConfirmAck(ConfirmAck::new_with_rebroadcasted_vote(
        vote.deref().clone(),
    ));
    assert_eq!(1, election.vote_count());

    node1.inbound_message_queue.put(con1, channel.info.clone());

    assert_timely_eq(Duration::from_secs(5), || election.vote_count(), 2);
}

#[test]
fn receivable_processor_confirm_sufficient_pos() {
    let mut system = System::new();
    let node1 = system.make_node();
    let send1 = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::raw(1),
        Account::zero().into(),
        &DEV_GENESIS_KEY,
        node1.work_generate_dev((*DEV_GENESIS_HASH).into()),
    ));

    node1.process(send1.clone()).unwrap();
    let election = start_election(&node1, &send1.hash());

    let vote = Arc::new(Vote::new_final(&DEV_GENESIS_KEY, vec![send1.hash()]));
    let channel = make_fake_channel(&node1);
    let con1 = Message::ConfirmAck(ConfirmAck::new_with_rebroadcasted_vote(
        vote.deref().clone(),
    ));
    assert_eq!(1, election.vote_count());

    node1.inbound_message_queue.put(con1, channel.info.clone());

    assert_timely_eq(Duration::from_secs(5), || election.vote_count(), 2);
}
