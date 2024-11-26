use rsnano_core::{Amount, Block, KeyPair, StateBlock, DEV_GENESIS_KEY};
use rsnano_ledger::{Writer, DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH, DEV_GENESIS_PUB_KEY};
use rsnano_node::{
    config::NodeFlags,
    consensus::{ActiveElectionsExt, Election, ElectionBehavior, ElectionStatus},
    stats::{DetailType, Direction, StatType},
};
use std::{sync::Arc, time::Duration};
use test_helpers::{assert_always_eq, assert_timely, assert_timely_eq, start_election, System};

#[test]
fn observer_callbacks() {
    let mut system = System::new();
    let config = System::default_config_without_backlog_population();
    let node = system.build_node().config(config).finish();
    node.insert_into_wallet(&DEV_GENESIS_KEY);
    let latest = node.latest(&DEV_GENESIS_ACCOUNT);

    let key1 = KeyPair::new();
    let send = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        latest,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::nano(1000),
        key1.account().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(latest),
    ));

    let send1 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        send.hash(),
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::nano(2000),
        key1.account().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(send.hash()),
    ));

    node.process_multi(&[send.clone(), send1.clone()]);

    node.confirming_set.add(send1.hash());

    // Callback is performed for all blocks that are confirmed
    assert_timely_eq(
        Duration::from_secs(5),
        || {
            node.stats
                .count_all(StatType::ConfirmationObserver, Direction::Out)
        },
        2,
    );

    assert_eq!(
        node.stats.count(
            StatType::ConfirmationHeight,
            DetailType::BlocksConfirmed,
            Direction::In
        ),
        2
    );
    assert_eq!(node.ledger.cemented_count(), 3);
    assert_eq!(
        node.active_elections
            .vote_applier
            .election_winner_details_len(),
        0
    );
}

// The callback and confirmation history should only be updated after confirmation height is set (and not just after voting)
#[test]
fn confirmed_history() {
    let mut system = System::new();
    let flags = NodeFlags {
        force_use_write_queue: true,
        disable_ascending_bootstrap: true,
        ..Default::default()
    };
    let config = System::default_config_without_backlog_population();
    let node = system.build_node().flags(flags).config(config).finish();
    let latest = node.latest(&DEV_GENESIS_ACCOUNT);

    let key1 = KeyPair::new();
    let send = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        latest,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::nano(1000),
        key1.account().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(latest),
    ));

    let send1 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        send.hash(),
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::nano(2000),
        key1.account().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(send.hash()),
    ));

    node.process_multi(&[send.clone(), send1.clone()]);

    let election = start_election(&node, &send1.hash());
    {
        // The write guard prevents the confirmation height processor doing any writes
        let _write_guard = node.ledger.write_queue.wait(Writer::Testing);

        // Confirm send1
        node.active_elections.force_confirm(&election);
        assert_timely_eq(Duration::from_secs(10), || node.active_elections.len(), 0);
        assert_eq!(node.active_elections.recently_cemented_count(), 0);
        assert_eq!(node.active_elections.len(), 0);

        let tx = node.ledger.read_txn();
        assert_eq!(
            node.ledger.confirmed().block_exists(&tx, &send.hash()),
            false
        );

        assert_timely(Duration::from_secs(10), || {
            node.ledger.write_queue.contains(Writer::ConfirmationHeight)
        });

        // Confirm that no inactive callbacks have been called when the
        // confirmation height processor has already iterated over it, waiting to write
        assert_always_eq(
            Duration::from_millis(50),
            || {
                node.stats.count(
                    StatType::ConfirmationObserver,
                    DetailType::InactiveConfHeight,
                    Direction::Out,
                )
            },
            0,
        );
    }

    assert_timely(Duration::from_secs(10), || {
        !node.ledger.write_queue.contains(Writer::ConfirmationHeight)
    });

    let tx = node.ledger.read_txn();
    assert!(node.ledger.confirmed().block_exists(&tx, &send.hash()));

    assert_timely_eq(Duration::from_secs(10), || node.active_elections.len(), 0);
    assert_timely_eq(
        Duration::from_secs(10),
        || {
            node.stats.count(
                StatType::ConfirmationObserver,
                DetailType::ActiveQuorum,
                Direction::Out,
            )
        },
        1,
    );

    // Each block that's confirmed is in the recently_cemented history
    assert_eq!(node.active_elections.recently_cemented_count(), 2);
    assert_eq!(node.active_elections.len(), 0);

    // Confirm the callback is not called under this circumstance
    assert_timely_eq(
        Duration::from_secs(5),
        || {
            node.stats.count(
                StatType::ConfirmationObserver,
                DetailType::ActiveQuorum,
                Direction::Out,
            )
        },
        1,
    );
    assert_timely_eq(
        Duration::from_secs(5),
        || {
            node.stats.count(
                StatType::ConfirmationObserver,
                DetailType::InactiveConfHeight,
                Direction::Out,
            )
        },
        1,
    );
    assert_timely_eq(
        Duration::from_secs(5),
        || {
            node.stats.count(
                StatType::ConfirmationHeight,
                DetailType::BlocksConfirmed,
                Direction::In,
            )
        },
        2,
    );
    assert_eq!(node.ledger.cemented_count(), 3);
    assert_eq!(
        node.active_elections
            .vote_applier
            .election_winner_details_len(),
        0
    );
}

#[test]
fn dependent_election() {
    let mut system = System::new();
    let flags = NodeFlags {
        force_use_write_queue: true,
        ..Default::default()
    };
    let config = System::default_config_without_backlog_population();
    let node = system.build_node().flags(flags).config(config).finish();
    let latest = node.latest(&DEV_GENESIS_ACCOUNT);

    let key1 = KeyPair::new();
    let send = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        latest,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::nano(1000),
        key1.account().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(latest),
    ));

    let send1 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        send.hash(),
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::nano(2000),
        key1.account().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(send.hash()),
    ));

    let send2 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        send1.hash(),
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::nano(3000),
        key1.account().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(send1.hash()),
    ));

    node.process_multi(&[send.clone(), send1.clone(), send2.clone()]);

    // This election should be confirmed as active_conf_height
    start_election(&node, &send1.hash());
    // Start an election and confirm it
    let election = start_election(&node, &send2.hash());
    node.active_elections.force_confirm(&election);

    // Wait for blocks to be confirmed in ledger, callbacks will happen after
    assert_timely_eq(
        Duration::from_secs(5),
        || {
            node.stats.count(
                StatType::ConfirmationHeight,
                DetailType::BlocksConfirmed,
                Direction::In,
            )
        },
        3,
    );
    // Once the item added to the confirming set no longer exists, callbacks have completed
    assert_timely(Duration::from_secs(5), || {
        !node.confirming_set.exists(&send2.hash())
    });

    assert_timely_eq(
        Duration::from_secs(5),
        || {
            node.stats.count(
                StatType::ConfirmationObserver,
                DetailType::ActiveQuorum,
                Direction::Out,
            )
        },
        1,
    );
    assert_timely_eq(
        Duration::from_secs(5),
        || {
            node.stats.count(
                StatType::ConfirmationObserver,
                DetailType::ActiveConfHeight,
                Direction::Out,
            )
        },
        1,
    );
    assert_timely_eq(
        Duration::from_secs(5),
        || {
            node.stats.count(
                StatType::ConfirmationObserver,
                DetailType::InactiveConfHeight,
                Direction::Out,
            )
        },
        1,
    );
    assert_eq!(node.ledger.cemented_count(), 4);
    assert_eq!(
        node.active_elections
            .vote_applier
            .election_winner_details_len(),
        0
    );
}

#[test]
fn election_winner_details_clearing_node_process_confirmed() {
    // Make sure election_winner_details is also cleared if the block never enters the confirmation height processor from node::process_confirmed
    let mut system = System::new();
    let node = system.make_node();

    let send = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::nano(1000),
        (*DEV_GENESIS_ACCOUNT).into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(*DEV_GENESIS_HASH),
    ));
    // Add to election_winner_details. Use an unrealistic iteration so that it should fall into the else case and do a cleanup
    node.active_elections
        .vote_applier
        .add_election_winner_details(
            send.hash(),
            Arc::new(Election::new(
                1,
                send.clone(),
                ElectionBehavior::Manual,
                Box::new(|_| {}),
                Box::new(|_| {}),
            )),
        );

    let mut election = ElectionStatus::default();
    election.winner = Some(send);

    node.active_elections.process_confirmed(election, 1000000);

    assert_eq!(
        node.active_elections
            .vote_applier
            .election_winner_details_len(),
        0
    );
}
