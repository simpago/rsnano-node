use crate::tests::helpers::{assert_never, assert_timely, assert_timely_eq, System};
use rsnano_core::{
    work::WorkPool, Amount, BlockEnum, KeyPair, RawKey, SendBlock, StateBlock, Vote,
    DEV_GENESIS_KEY,
};
use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH};
use rsnano_messages::{ConfirmAck, Message};
use rsnano_node::{
    stats::{DetailType, Direction, StatType},
    transport::{BufferDropPolicy, PeerConnectorExt, TrafficType},
    wallets::WalletsExt,
};
use std::time::Duration;
use tracing::error;

#[test]
fn local_block_broadcast() {
    let mut system = System::new();

    let mut node_config = System::default_config();
    node_config.priority_scheduler_enabled = false;
    node_config.hinted_scheduler.enabled = false;
    node_config.optimistic_scheduler.enabled = false;
    node_config.local_block_broadcaster.rebroadcast_interval = Duration::from_secs(1);

    let node1 = system.build_node().config(node_config).finish();
    let node2 = system.make_disconnected_node();

    let key1 = KeyPair::new();
    let latest_hash = *DEV_GENESIS_HASH;

    let send1 = BlockEnum::LegacySend(SendBlock::new(
        &latest_hash,
        &key1.public_key(),
        &(Amount::MAX - Amount::nano(1000)),
        &DEV_GENESIS_KEY.private_key(),
        &DEV_GENESIS_KEY.public_key(),
        system.work.generate_dev2(latest_hash.into()).unwrap(),
    ));

    let qualified_root = send1.qualified_root();
    let send_hash = send1.hash();
    node1.process_local(send1).unwrap();

    assert_never(Duration::from_millis(500), || {
        node1.active.active_root(&qualified_root)
    });

    // Wait until a broadcast is attempted
    assert_timely_eq(
        Duration::from_secs(5),
        || node1.local_block_broadcaster.len(),
        1,
    );
    assert_timely(
        Duration::from_secs(5),
        || {
            node1.stats.count(
                StatType::LocalBlockBroadcaster,
                DetailType::Broadcast,
                Direction::Out,
            ) >= 1
        },
        "no broadcast sent",
    );

    // The other node should not have received a block
    assert_never(Duration::from_millis(500), || {
        node2.block(&send_hash).is_some()
    });

    error!(
        "node2 local addr is {:?}",
        node2.tcp_listener.local_address()
    );

    // Connect the nodes and check that the block is propagated
    node1
        .peer_connector
        .connect_to(node2.tcp_listener.local_address());
    assert_timely(
        Duration::from_secs(5),
        || node1.network.find_node_id(&node2.get_node_id()).is_some(),
        "node2 not connected",
    );
    assert_timely(
        Duration::from_secs(10),
        || node2.block(&send_hash).is_some(),
        "block not received",
    )
}

#[test]
fn fork_no_vote_quorum() {
    let mut system = System::new();
    let node1 = system.make_node();
    let node2 = system.make_node();
    let node3 = system.make_node();
    let wallet_id1 = node1.wallets.wallet_ids()[0];
    let wallet_id2 = node2.wallets.wallet_ids()[0];
    let wallet_id3 = node3.wallets.wallet_ids()[0];
    node1
        .wallets
        .insert_adhoc2(&wallet_id1, &DEV_GENESIS_KEY.private_key(), true)
        .unwrap();
    let key4 = node1
        .wallets
        .deterministic_insert2(&wallet_id1, true)
        .unwrap();
    node1
        .wallets
        .send_action2(
            &wallet_id1,
            *DEV_GENESIS_ACCOUNT,
            key4,
            Amount::MAX / 4,
            0,
            true,
            None,
        )
        .unwrap();
    let key1 = node2
        .wallets
        .deterministic_insert2(&wallet_id2, true)
        .unwrap();
    node2
        .wallets
        .set_representative(wallet_id2, key1, false)
        .unwrap();
    let block = node1
        .wallets
        .send_action2(
            &wallet_id1,
            *DEV_GENESIS_ACCOUNT,
            key1,
            node1.config.receive_minimum,
            0,
            true,
            None,
        )
        .unwrap();
    assert_timely(
        Duration::from_secs(30),
        || {
            node3.balance(&key1) == node1.config.receive_minimum
                && node2.balance(&key1) == node1.config.receive_minimum
                && node1.balance(&key1) == node1.config.receive_minimum
        },
        "balances are wrong",
    );
    assert_eq!(node1.config.receive_minimum, node1.ledger.weight(&key1));
    assert_eq!(node1.config.receive_minimum, node2.ledger.weight(&key1));
    assert_eq!(node1.config.receive_minimum, node3.ledger.weight(&key1));

    let send1 = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        block.hash(),
        *DEV_GENESIS_ACCOUNT,
        (Amount::MAX / 4) - (node1.config.receive_minimum * 2),
        key1.into(),
        &DEV_GENESIS_KEY,
        node1.work_generate_dev(block.hash().into()),
    ));

    node1.process(send1.clone()).unwrap();
    node2.process(send1.clone()).unwrap();
    node3.process(send1.clone()).unwrap();

    let key2 = node3
        .wallets
        .deterministic_insert2(&wallet_id3, true)
        .unwrap();

    let send2 = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        block.hash(),
        *DEV_GENESIS_ACCOUNT,
        (Amount::MAX / 4) - (node1.config.receive_minimum * 2),
        key2.into(),
        &DEV_GENESIS_KEY,
        node1.work_generate_dev(block.hash().into()),
    ));
    let key3 = RawKey::random();
    let vote = Vote::new(key1, &key3, 0, 0, vec![send2.hash()]);
    let confirm = Message::ConfirmAck(ConfirmAck::new_with_own_vote(vote));
    let channel = node2
        .network
        .find_node_id(&node3.node_id.public_key())
        .unwrap();
    channel.try_send(
        &confirm,
        BufferDropPolicy::NoLimiterDrop,
        TrafficType::Generic,
    );

    assert_timely(
        Duration::from_secs(10),
        || {
            node3
                .stats
                .count(StatType::Message, DetailType::ConfirmAck, Direction::In)
                >= 3
        },
        "no confirm ack",
    );
    assert_eq!(node1.latest(&DEV_GENESIS_ACCOUNT), send1.hash());
    assert_eq!(node2.latest(&DEV_GENESIS_ACCOUNT), send1.hash());
    assert_eq!(node3.latest(&DEV_GENESIS_ACCOUNT), send1.hash());
}
