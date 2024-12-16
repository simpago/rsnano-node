use rsnano_core::{Amount, PrivateKey, UnsavedBlockLatticeBuilder, DEV_GENESIS_KEY};
use rsnano_ledger::Writer;
use rsnano_node::{
    config::NodeFlags,
    consensus::ActiveElectionsExt,
    stats::{DetailType, Direction, StatType},
};
use std::time::Duration;
use test_helpers::{assert_always_eq, assert_timely, assert_timely_eq, start_election, System};

#[test]
fn observer_callbacks() {
    let mut system = System::new();
    let config = System::default_config_without_backlog_population();
    let node = system.build_node().config(config).finish();
    node.insert_into_wallet(&DEV_GENESIS_KEY);

    let mut lattice = UnsavedBlockLatticeBuilder::new();
    let key1 = PrivateKey::new();
    let send = lattice.genesis().send(&key1, Amount::nano(1000));
    let send1 = lattice.genesis().send(&key1, Amount::nano(1000));
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
}

// The callback and confirmation history should only be updated after confirmation height is set (and not just after voting)
#[test]
fn confirmed_history() {
    let mut system = System::new();
    let mut config = System::default_config_without_backlog_population();
    config.bootstrap.enable = false;
    let node = system.build_node().config(config).finish();

    let mut lattice = UnsavedBlockLatticeBuilder::new();
    let key1 = PrivateKey::new();
    let send = lattice.genesis().send(&key1, Amount::nano(1000));
    let send1 = lattice.genesis().send(&key1, Amount::nano(1000));

    node.process_multi(&[send.clone(), send1.clone()]);

    let election = start_election(&node, &send1.hash());
    {
        // The write guard prevents the confirmation height processor doing any writes
        let _write_guard = node.ledger.write_queue.wait(Writer::Testing);

        // Confirm send1
        node.active.force_confirm(&election);
        assert_timely_eq(Duration::from_secs(10), || node.active.len(), 0);
        assert_eq!(node.active.recently_cemented_count(), 0);
        assert_eq!(node.active.len(), 0);

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

    assert_timely(Duration::from_secs(5), || {
        node.ledger
            .confirmed()
            .block_exists(&node.ledger.read_txn(), &send.hash())
    });

    assert_timely_eq(Duration::from_secs(10), || node.active.len(), 0);
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
    assert_eq!(node.active.recently_cemented_count(), 2);
    assert_eq!(node.active.len(), 0);

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
}

#[test]
fn dependent_election() {
    let mut system = System::new();
    let config = System::default_config_without_backlog_population();
    let node = system.build_node().config(config).finish();

    let mut lattice = UnsavedBlockLatticeBuilder::new();
    let key1 = PrivateKey::new();
    let send = lattice.genesis().send(&key1, Amount::nano(1000));
    let send1 = lattice.genesis().send(&key1, Amount::nano(1000));
    let send2 = lattice.genesis().send(&key1, Amount::nano(1000));
    node.process_multi(&[send.clone(), send1.clone(), send2.clone()]);

    // This election should be confirmed as active_conf_height
    start_election(&node, &send1.hash());
    // Start an election and confirm it
    let election = start_election(&node, &send2.hash());
    node.active.force_confirm(&election);

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
        !node.confirming_set.contains(&send2.hash())
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
}
