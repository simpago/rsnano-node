use rsnano_core::{
    Account, Amount, Block, BlockHash, PrivateKey, UncheckedKey, UnsavedBlockLatticeBuilder,
    WalletId, DEV_GENESIS_KEY,
};
use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH};
use rsnano_messages::BulkPull;
use rsnano_network::{
    bandwidth_limiter::BandwidthLimiter, Channel, ChannelInfo, NullNetworkObserver,
};
use rsnano_node::{
    bootstrap::{BootstrapAttemptTrait, BootstrapInitiatorExt, BootstrapStrategy, BulkPullServer},
    config::{NodeConfig, NodeFlags},
    stats::{DetailType, Direction, StatType},
    transport::{LatestKeepalives, ResponseServer},
    wallets::WalletsExt,
    Node, NodeExt,
};
use rsnano_nullable_tcp::TcpStream;
use std::sync::{atomic::Ordering, Arc, Mutex};
use std::time::Duration;
use test_helpers::{
    assert_timely, assert_timely_eq, assert_timely_msg, get_available_port, setup_chain, System,
};

mod bootstrap_processor {
    use super::*;
    use rsnano_core::UnsavedBlockLatticeBuilder;
    use rsnano_ledger::BlockStatus;
    use rsnano_network::ChannelMode;
    use test_helpers::establish_tcp;

    #[test]
    fn bootstrap_processor_wallet_lazy_pending() {
        let mut system = System::new();
        let mut config = System::default_config_without_backlog_population();
        let mut flags = NodeFlags::new();
        flags.disable_bootstrap_bulk_push_client = true;
        flags.disable_legacy_bootstrap = true;
        flags.disable_ascending_bootstrap = true;
        flags.disable_ongoing_bootstrap = true;
        let node0 = system
            .build_node()
            .config(config.clone())
            .flags(flags.clone())
            .finish();

        let mut lattice = UnsavedBlockLatticeBuilder::new();
        let key1 = PrivateKey::new();
        let key2 = PrivateKey::new();
        // Generating test chain

        let send1 = lattice.genesis().send(&key1, Amount::nano(1000));
        let receive1 = lattice.account(&key1).receive(&send1);
        let send2 = lattice.account(&key1).send_max(&key2);

        // Processing test chain
        node0.process_multi(&[send1.clone(), receive1.clone(), send2.clone()]);
        assert_timely_msg(
            Duration::from_secs(5),
            || node0.blocks_exist(&[send1.clone(), receive1.clone(), send2.clone()]),
            "blocks not processed",
        );

        // Start wallet lazy bootstrap
        config.peering_port = Some(get_available_port());
        let node1 = system.build_node().config(config).flags(flags).finish();
        let wallet_id = WalletId::random();
        node1.wallets.create(wallet_id);
        node1
            .wallets
            .insert_adhoc2(&wallet_id, &key2.raw_key(), true)
            .unwrap();
        node1.bootstrap_wallet();

        // Check processed blocks
        assert_timely(Duration::from_secs(10), || {
            node1.block_exists(&send2.hash())
        });
    }

    #[test]
    fn bootstrap_processor_lazy_destinations() {
        let mut system = System::new();
        let mut config = System::default_config_without_backlog_population();
        let mut flags = NodeFlags::new();
        flags.disable_bootstrap_bulk_push_client = true;
        flags.disable_legacy_bootstrap = true;
        flags.disable_ascending_bootstrap = true;
        flags.disable_ongoing_bootstrap = true;
        let node1 = system
            .build_node()
            .config(config.clone())
            .flags(flags.clone())
            .finish();

        let mut lattice = UnsavedBlockLatticeBuilder::new();
        let key1 = PrivateKey::new();
        let key2 = PrivateKey::new();

        // send 1000 nano from genesis to key1
        let send1 = lattice.genesis().legacy_send(&key1, Amount::nano(1000));
        assert_eq!(
            node1.process_local(send1.clone()).unwrap(),
            BlockStatus::Progress
        );

        // send 1000 nano from genesis to key2
        let send2 = lattice.genesis().send(&key2, Amount::nano(1000));

        assert_eq!(
            node1.process_local(send2.clone()).unwrap(),
            BlockStatus::Progress
        );

        // receive send1 on key1
        let open = lattice.account(&key1).legacy_open(&send1);

        assert_eq!(
            node1.process_local(open.clone()).unwrap(),
            BlockStatus::Progress
        );

        // receive send2 on key2
        let state_open = lattice.account(&key2).receive(&send2);

        assert_eq!(
            node1.process_local(state_open.clone()).unwrap(),
            BlockStatus::Progress
        );

        // Start lazy bootstrap with last block in sender chain
        config.peering_port = Some(get_available_port());
        let node2 = system.build_node().config(config).flags(flags).finish();
        node2
            .bootstrap_initiator
            .bootstrap_lazy(send2.hash().into(), true, "".to_string());

        // Check processed blocks
        assert_timely(Duration::from_secs(5), || {
            !node2.bootstrap_initiator.in_progress()
        });
        assert_timely(Duration::from_secs(5), || node2.block_exists(&send1.hash()));
        assert_timely(Duration::from_secs(5), || node2.block_exists(&send2.hash()));
        assert!(!node2.block_exists(&open.hash()));
        assert!(!node2.block_exists(&state_open.hash()));

        node2.stop();
    }

    #[test]
    fn bootstrap_processor_lazy_unclear_state_link_not_existing() {
        let mut system = System::new();
        let mut config = System::default_config_without_backlog_population();
        let mut flags = NodeFlags::new();
        flags.disable_bootstrap_bulk_push_client = true;
        flags.disable_legacy_bootstrap = true;
        flags.disable_ascending_bootstrap = true;
        flags.disable_ongoing_bootstrap = true;
        let node1 = system
            .build_node()
            .config(config.clone())
            .flags(flags.clone())
            .finish();

        let mut lattice = UnsavedBlockLatticeBuilder::new();
        let key = PrivateKey::new();
        let key2 = PrivateKey::new();
        // Generating test chain

        let send1 = lattice.genesis().legacy_send(&key, Amount::nano(1000));

        assert_eq!(
            node1.process_local(send1.clone()).unwrap(),
            BlockStatus::Progress
        );

        let open = lattice.account(&key).legacy_open(&send1);

        assert_eq!(
            node1.process_local(open.clone()).unwrap(),
            BlockStatus::Progress
        );

        let send2 = lattice.account(&key).send_max(&key2);

        assert_eq!(
            node1.process_local(send2.clone()).unwrap(),
            BlockStatus::Progress
        );

        // Start lazy bootstrap with last block in chain known
        config.peering_port = Some(get_available_port());
        let node2 = system.build_node().config(config).flags(flags).finish();
        node2
            .bootstrap_initiator
            .bootstrap_lazy(send2.hash().into(), true, "".to_string());

        // Check processed blocks
        assert_timely(Duration::from_secs(15), || {
            !node2.bootstrap_initiator.in_progress()
        });
        assert_timely_msg(
            Duration::from_secs(15),
            || node2.block_hashes_exist(vec![send1.hash(), open.hash(), send2.hash()]),
            "blocks not bootstrapped",
        );

        assert_eq!(
            node2.stats.count(
                StatType::Bootstrap,
                DetailType::BulkPullFailedAccount,
                Direction::In
            ),
            1
        );

        node2.stop();
    }

    #[test]
    fn bootstrap_processor_lazy_unclear_state_link() {
        let mut system = System::new();
        let mut config = System::default_config_without_backlog_population();
        let mut flags = NodeFlags::new();
        flags.disable_bootstrap_bulk_push_client = true;
        flags.disable_legacy_bootstrap = true;
        flags.disable_ascending_bootstrap = true;
        flags.disable_ongoing_bootstrap = true;

        let node1 = system
            .build_node()
            .config(config.clone())
            .flags(flags.clone())
            .finish();

        let mut lattice = UnsavedBlockLatticeBuilder::new();
        let key = PrivateKey::new();

        let send1 = lattice.genesis().send(&key, Amount::nano(1000));
        assert_eq!(
            node1.process_local(send1.clone()).unwrap(),
            BlockStatus::Progress
        );

        let send2 = lattice.genesis().send(&key, Amount::nano(1000));
        assert_eq!(
            node1.process_local(send2.clone()).unwrap(),
            BlockStatus::Progress
        );

        let open = lattice.account(&key).receive(&send1);
        assert_eq!(
            node1.process_local(open.clone()).unwrap(),
            BlockStatus::Progress
        );

        let receive = lattice.account(&key).receive(&send2);
        assert_eq!(
            node1.process_local(receive.clone()).unwrap(),
            BlockStatus::Progress
        );

        assert_timely_msg(
            Duration::from_secs(5),
            || node1.blocks_exist(&[send1.clone(), send2.clone(), open.clone(), receive.clone()]),
            "blocks not processed",
        );

        // Start lazy bootstrap with last block in chain known
        config.peering_port = Some(get_available_port());
        let node2 = system.build_node().config(config).flags(flags).finish();
        node2
            .bootstrap_initiator
            .bootstrap_lazy(receive.hash().into(), true, "".to_string());

        assert_timely_msg(
            Duration::from_secs(5),
            || node2.blocks_exist(&[send1.clone(), send2.clone(), open.clone(), receive.clone()]),
            "blocks not bootstrapped",
        );

        assert_eq!(
            node2.stats.count(
                StatType::Bootstrap,
                DetailType::BulkPullFailedAccount,
                Direction::In
            ),
            0
        );
    }

    #[test]
    fn bootstrap_processor_lazy_hash_pruning() {
        let mut system = System::new();
        let mut config = System::default_config_without_backlog_population();
        config.enable_voting = false; // Remove after allowing pruned voting

        let mut flags = NodeFlags::new();
        flags.disable_bootstrap_bulk_push_client = true;
        flags.enable_pruning = true;

        let node0 = system
            .build_node()
            .config(config.clone())
            .flags(flags.clone())
            .finish();

        let mut lattice = UnsavedBlockLatticeBuilder::new();
        let key1 = PrivateKey::new();
        let key2 = PrivateKey::new();

        // send 1000 nano from genesis to genesis
        let send1 = lattice
            .genesis()
            .send(*DEV_GENESIS_ACCOUNT, Amount::nano(1000));

        let receive1 = lattice.genesis().receive(&send1);
        let change1 = lattice.genesis().change(&key1);
        let change2 = lattice.genesis().change(&key2);

        // send 1000 nano from genesis to key1 and genesis rep back to genesis account
        let send2 = lattice.genesis().send(&key1, Amount::nano(1000));

        // receive send2 and rep of key1 to be itself
        let receive2 = lattice.account(&key1).receive(&send2);

        // send 1000 nano, all available balance, from key1 to key2
        let send3 = lattice.account(&key1).send(&key2, Amount::nano(1000));

        // receive send3 on key2, set rep of key2 to be itself
        let receive3 = lattice.account(&key2).receive(&send3);

        let blocks = vec![
            send1.clone(),
            receive1.clone(),
            change1.clone(),
            change2.clone(),
            send2,
            receive2,
            send3,
            receive3.clone(),
        ];
        node0.process_multi(&blocks);
        node0.confirm_multi(&blocks);

        config.peering_port = Some(get_available_port());
        let node1 = system
            .build_node()
            .config(config)
            .flags(flags)
            .disconnected()
            .finish();

        // Processing chain to prune for node1
        node1.process_active(send1.clone());
        node1.process_active(receive1.clone());
        node1.process_active(change1.clone());
        node1.process_active(change2.clone());
        assert_timely_msg(
            Duration::from_secs(5),
            || {
                node1.blocks_exist(&[
                    send1.clone(),
                    receive1.clone(),
                    change1.clone(),
                    change2.clone(),
                ])
            },
            "blocks not processed",
        );

        // Confirm last block to prune previous
        node1.confirm_multi(&[send1, receive1, change1, change2]);
        assert_eq!(5, node1.ledger.block_count());
        assert_eq!(5, node1.ledger.cemented_count());

        // Pruning action
        node1.ledger_pruning(2, false);
        assert_eq!(9, node0.ledger.block_count());
        assert_eq!(0, node0.ledger.pruned_count());
        assert_eq!(5, node1.ledger.block_count());
        assert_eq!(3, node1.ledger.pruned_count());

        // Start lazy bootstrap with last block in chain known
        establish_tcp(&node1, &node0);
        node1
            .bootstrap_initiator
            .bootstrap_lazy(receive3.hash().into(), true, "".to_string());

        // Check processed blocks
        assert_timely_eq(Duration::from_secs(5), || node1.ledger.block_count(), 9);
        assert_timely(Duration::from_secs(5), || {
            node1.balance(&key2.account()) != Amount::zero()
        });
        assert_timely(Duration::from_secs(5), || {
            !node1.bootstrap_initiator.in_progress()
        });
    }

    #[test]
    fn bootstrap_processor_pull_diamond() {
        let mut system = System::new();
        let config = System::default_config_without_backlog_population();
        let mut flags = NodeFlags::new();
        flags.disable_bootstrap_bulk_push_client = true;
        let node0 = system.build_node().config(config).flags(flags).finish();

        let mut lattice = UnsavedBlockLatticeBuilder::new();
        let key = PrivateKey::new();

        let send1 = lattice.genesis().send_max(&key);
        assert_eq!(
            node0.process_local(send1.clone()).unwrap(),
            BlockStatus::Progress
        );

        let open = lattice.account(&key).receive(&send1);
        assert_eq!(
            node0.process_local(open.clone()).unwrap(),
            BlockStatus::Progress
        );

        let send2 = lattice.account(&key).send(&*DEV_GENESIS_KEY, 100);
        assert_eq!(
            node0.process_local(send2.clone()).unwrap(),
            BlockStatus::Progress
        );

        let receive = lattice.genesis().receive(&send2);
        assert_eq!(
            node0.process_local(receive.clone()).unwrap(),
            BlockStatus::Progress
        );

        let node1 = system.make_disconnected_node();
        node1
            .peer_connector
            .connect_to(node0.tcp_listener.local_address());
        node1
            .bootstrap_initiator
            .bootstrap2(node0.tcp_listener.local_address(), "".into());
        assert_timely_eq(
            Duration::from_secs(5),
            || node1.balance(&DEV_GENESIS_ACCOUNT),
            Amount::raw(100),
        );
    }

    // Bootstrap can pull universal blocks
    #[test]
    fn bootstrap_processor_process_state() {
        let mut system = System::new();
        let config = System::default_config_without_backlog_population();
        let mut flags = NodeFlags::new();
        flags.disable_bootstrap_bulk_push_client = true;
        let node0 = system
            .build_node()
            .config(config.clone())
            .flags(flags.clone())
            .finish();

        node0.insert_into_wallet(&DEV_GENESIS_KEY);

        let mut lattice = UnsavedBlockLatticeBuilder::new();
        let send = lattice.genesis().send(*DEV_GENESIS_ACCOUNT, 100);
        let receive = lattice.genesis().receive(&send);
        assert_eq!(
            node0.process_local(send.clone()).unwrap(),
            BlockStatus::Progress
        );
        assert_eq!(
            node0.process_local(receive.clone()).unwrap(),
            BlockStatus::Progress
        );

        assert_timely_eq(
            Duration::from_secs(5),
            || {
                node0
                    .ledger
                    .account_info(&node0.ledger.read_txn(), &DEV_GENESIS_ACCOUNT)
                    .map(|info| info.block_count)
                    .unwrap_or(0)
            },
            3,
        );

        let node1 = system.build_node().flags(flags).disconnected().finish();
        assert_eq!(node0.latest(&DEV_GENESIS_ACCOUNT), receive.hash());
        assert_ne!(node1.latest(&DEV_GENESIS_ACCOUNT), receive.hash());

        node1
            .peer_connector
            .connect_to(node0.tcp_listener.local_address());
        node1
            .bootstrap_initiator
            .bootstrap2(node0.tcp_listener.local_address(), "".into());

        assert_timely_eq(
            Duration::from_secs(5),
            || node1.latest(&DEV_GENESIS_ACCOUNT),
            receive.hash(),
        );
    }

    #[test]
    fn bootstrap_processor_process_new() {
        let mut system = System::new();
        let mut config = System::default_config_without_backlog_population();
        let mut flags = NodeFlags::new();
        flags.disable_bootstrap_bulk_push_client = true;
        let key2 = PrivateKey::new();

        let node1 = system
            .build_node()
            .config(config.clone())
            .flags(flags.clone())
            .finish();
        config.peering_port = Some(get_available_port());
        let node2 = system.build_node().config(config).flags(flags).finish();

        let wallet_id1 = node1.wallets.wallet_ids()[0];
        node1.insert_into_wallet(&DEV_GENESIS_KEY);
        node2.insert_into_wallet(&key2);

        // send amount raw from genesis to key2, the wallet will autoreceive
        let amount = node1.config.receive_minimum;
        let send = node1
            .wallets
            .send_action2(
                &wallet_id1,
                *DEV_GENESIS_ACCOUNT,
                key2.public_key().into(),
                amount,
                0,
                true,
                None,
            )
            .unwrap();

        assert_timely(Duration::from_secs(5), || {
            !node1.balance(&key2.public_key().into()).is_zero()
        });

        // wait for the receive block on node2
        assert_timely(Duration::from_secs(5), || {
            node2.block_exists(&node2.latest(&key2.public_key().into()))
        });

        let receive = node2
            .block(&node2.latest(&key2.public_key().into()))
            .unwrap();

        // All blocks should be propagated & confirmed
        assert_timely(Duration::from_secs(5), || {
            node1.block_hashes_confirmed(&[send.hash(), receive.hash()])
        });
        assert_timely(Duration::from_secs(5), || {
            node2.block_hashes_confirmed(&[send.hash(), receive.hash()])
        });
        assert_timely(Duration::from_secs(5), || node1.active.len() == 0);
        assert_timely(Duration::from_secs(5), || node2.active.len() == 0);

        // create a node manually to avoid making automatic network connections
        let node3 = system.make_disconnected_node();
        node3
            .peer_connector
            .connect_to(node1.tcp_listener.local_address());
        node3
            .bootstrap_initiator
            .bootstrap2(node1.tcp_listener.local_address(), "".into());
        assert_timely_eq(
            Duration::from_secs(5),
            || node3.balance(&key2.public_key().into()),
            amount,
        );
    }

    #[test]
    fn bootstrap_processor_process_two() {
        let mut system = System::new();
        let config = System::default_config_without_backlog_population();
        let mut flags = NodeFlags::new();
        flags.disable_bootstrap_bulk_push_client = true;
        let node0 = system.build_node().config(config).flags(flags).finish();

        node0.insert_into_wallet(&DEV_GENESIS_KEY);
        let wallet_id = node0.wallets.wallet_ids()[0];

        node0
            .wallets
            .send_action2(
                &wallet_id,
                *DEV_GENESIS_ACCOUNT,
                *DEV_GENESIS_ACCOUNT,
                Amount::raw(50),
                0,
                true,
                None,
            )
            .unwrap();

        node0
            .wallets
            .send_action2(
                &wallet_id,
                *DEV_GENESIS_ACCOUNT,
                *DEV_GENESIS_ACCOUNT,
                Amount::raw(50),
                0,
                true,
                None,
            )
            .unwrap();

        assert_timely_eq(
            Duration::from_secs(5),
            || {
                node0
                    .ledger
                    .account_info(&node0.ledger.read_txn(), &DEV_GENESIS_ACCOUNT)
                    .map(|info| info.block_count)
                    .unwrap_or(0)
            },
            3,
        );

        // Create a node manually to avoid making automatic network connections
        let node1 = system.make_disconnected_node();

        // Nodes should be out of sync here
        assert_ne!(
            node1.latest(&DEV_GENESIS_ACCOUNT),
            node0.latest(&DEV_GENESIS_ACCOUNT)
        );

        node1
            .peer_connector
            .connect_to(node0.tcp_listener.local_address());
        // Bootstrap triggered
        node1
            .bootstrap_initiator
            .bootstrap2(node0.tcp_listener.local_address(), "".into());

        // Nodes should sync up
        assert_timely_eq(
            Duration::from_secs(5),
            || node1.latest(&DEV_GENESIS_ACCOUNT),
            node0.latest(&DEV_GENESIS_ACCOUNT),
        );
    }

    // Bootstrap can pull one basic block
    #[test]
    fn bootstrap_processor_process_one() {
        let mut system = System::new();
        let mut config = System::default_config_without_backlog_population();
        config.enable_voting = false;
        let mut flags = NodeFlags::new();
        flags.disable_bootstrap_bulk_push_client = true;
        let node0 = system
            .build_node()
            .config(config.clone())
            .flags(flags.clone())
            .finish();

        node0.insert_into_wallet(&DEV_GENESIS_KEY);
        let wallet_id = node0.wallets.wallet_ids()[0];
        node0
            .wallets
            .send_action2(
                &wallet_id,
                *DEV_GENESIS_ACCOUNT,
                *DEV_GENESIS_ACCOUNT,
                Amount::raw(100),
                0,
                true,
                None,
            )
            .unwrap();

        assert_timely(Duration::from_secs(5), || {
            node0.latest(&DEV_GENESIS_ACCOUNT) != *DEV_GENESIS_HASH
        });

        flags.disable_rep_crawler = true;
        config.peering_port = Some(get_available_port());
        let node1 = system
            .build_node()
            .config(config)
            .flags(flags)
            .disconnected()
            .finish();

        assert_ne!(
            node0.latest(&DEV_GENESIS_ACCOUNT),
            node1.latest(&DEV_GENESIS_ACCOUNT)
        );

        node1
            .peer_connector
            .connect_to(node0.tcp_listener.local_address());
        node1
            .bootstrap_initiator
            .bootstrap2(node0.tcp_listener.local_address(), "".into());

        assert_timely_eq(
            Duration::from_secs(10),
            || node1.latest(&DEV_GENESIS_ACCOUNT),
            node0.latest(&DEV_GENESIS_ACCOUNT),
        );
    }

    #[test]
    fn bootstrap_processor_lazy_hash() {
        let mut system = System::new();
        let config = System::default_config_without_backlog_population();
        let mut flags = NodeFlags::new();
        flags.disable_bootstrap_bulk_push_client = true;
        let node0 = system.build_node().config(config).flags(flags).finish();

        let mut lattice = UnsavedBlockLatticeBuilder::new();
        let key1 = PrivateKey::new();
        let key2 = PrivateKey::new();

        let send1 = lattice.genesis().send(&key1, Amount::nano(1000));
        let receive1 = lattice.account(&key1).receive(&send1);
        let send2 = lattice.account(&key1).send(&key2, Amount::nano(1000));
        let receive2 = lattice.account(&key2).receive(&send2);

        // Processing test chain
        let blocks = [send1, receive1, send2, receive2.clone()];
        node0.process_multi(&blocks);

        assert_timely_msg(
            Duration::from_secs(5),
            || node0.blocks_exist(&blocks),
            "blocks not processed",
        );

        // Start lazy bootstrap with last block in chain known
        let node1 = system.make_disconnected_node();
        establish_tcp(&node1, &node0);
        node1
            .bootstrap_initiator
            .bootstrap_lazy(receive2.hash().into(), true, "".to_string());

        {
            let lazy_attempt = node1
                .bootstrap_initiator
                .current_lazy_attempt()
                .expect("no lazy attempt found");

            let BootstrapStrategy::Lazy(lazy) = lazy_attempt.as_ref() else {
                panic!("not lazy")
            };
            assert_eq!(lazy.id(), receive2.hash().to_string());
        }

        // Check processed blocks
        assert_timely_eq(
            Duration::from_secs(10),
            || node1.balance(&key2.account()),
            Amount::nano(1000),
        );
    }

    #[test]
    fn bootstrap_processor_lazy_hash_bootstrap_id() {
        let mut system = System::new();
        let config = System::default_config_without_backlog_population();
        let mut flags = NodeFlags::new();
        flags.disable_bootstrap_bulk_push_client = true;
        let node0 = system.build_node().config(config).flags(flags).finish();

        let mut lattice = UnsavedBlockLatticeBuilder::new();
        let key1 = PrivateKey::new();
        let key2 = PrivateKey::new();
        // Generating test chain

        let send1 = lattice.genesis().send(&key1, Amount::nano(1000));
        let receive1 = lattice.account(&key1).receive(&send1);
        let send2 = lattice.account(&key1).send(&key2, Amount::nano(1000));
        let receive2 = lattice.account(&key2).receive(&send2);

        // Processing test chain
        let blocks = [send1, receive1, send2, receive2.clone()];
        node0.process_multi(&blocks);

        assert_timely_msg(
            Duration::from_secs(5),
            || node0.blocks_exist(&blocks),
            "blocks not processed",
        );

        // Start lazy bootstrap with last block in chain known
        let node1 = system.make_disconnected_node();
        establish_tcp(&node1, &node0);
        node1.bootstrap_initiator.bootstrap_lazy(
            receive2.hash().into(),
            true,
            "123456".to_string(),
        );

        {
            let lazy_attempt = node1
                .bootstrap_initiator
                .current_lazy_attempt()
                .expect("no lazy attempt found");

            let BootstrapStrategy::Lazy(lazy) = lazy_attempt.as_ref() else {
                panic!("not lazy")
            };
            assert_eq!(lazy.id(), "123456".to_string());
        }

        // Check processed blocks
        assert_timely_eq(
            Duration::from_secs(10),
            || node1.balance(&key2.account()),
            Amount::nano(1000),
        );
    }

    #[test]
    #[ignore = "fails because of an unknown bug. pruning has low priority right now"]
    fn bootstrap_processor_lazy_pruning_missing_block() {
        let mut system = System::new();
        let mut config = System::default_config_without_backlog_population();
        config.enable_voting = false; // Remove after allowing pruned voting

        let mut flags = NodeFlags::new();
        flags.disable_bootstrap_bulk_push_client = true;
        flags.disable_legacy_bootstrap = true;
        flags.disable_ascending_bootstrap = true;
        flags.disable_ongoing_bootstrap = true;
        flags.enable_pruning = true;

        let node1 = system
            .build_node()
            .config(config.clone())
            .flags(flags.clone())
            .finish();

        let mut lattice = UnsavedBlockLatticeBuilder::new();
        let key1 = PrivateKey::new();
        let key2 = PrivateKey::new();

        let send1 = lattice.genesis().send(&key1, Amount::nano(1000));
        let send2 = lattice.genesis().send(&key2, Amount::nano(1000));
        let receive1 = lattice.account(&key1).receive(&send1);
        let receive2 = lattice.account(&key2).receive(&send2);

        // add the blocks without starting elections because elections publish blocks
        // and the publishing would interefere with the testing
        let blocks = [send1.clone(), send2.clone(), receive1, receive2];
        node1.process_multi(&blocks);

        assert_timely_msg(
            Duration::from_secs(5),
            || node1.blocks_exist(&blocks),
            "blocks not processed",
        );

        node1.confirm_multi(&blocks);

        assert_timely_msg(
            Duration::from_secs(5),
            || node1.blocks_confirmed(&blocks),
            "blocks not confirmed",
        );

        // Pruning action, send1 should get pruned
        node1.ledger_pruning(2, false);
        assert_eq!(1, node1.ledger.pruned_count());
        assert_eq!(5, node1.ledger.block_count());
        assert!(node1
            .ledger
            .store
            .pruned
            .exists(&node1.ledger.read_txn(), &send1.hash()));

        // Start lazy bootstrap with last block in sender chain
        config.peering_port = Some(get_available_port());
        let node2 = system
            .build_node()
            .config(config)
            .flags(flags)
            .disconnected()
            .finish();

        establish_tcp(&node2, &node1);
        node2
            .bootstrap_initiator
            .bootstrap_lazy(send2.hash().into(), false, "".to_string());

        // Check processed blocks
        let lazy_attempt = node2
            .bootstrap_initiator
            .current_lazy_attempt()
            .expect("no lazy attempt");

        assert_timely_msg(
            Duration::from_secs(5),
            || lazy_attempt.stopped() || lazy_attempt.requeued_pulls() >= 4,
            "did not stop",
        );

        // Some blocks cannot be retrieved from pruned node
        assert_eq!(node1.block_hashes_exist([send1.hash()]), false);
        assert_eq!(node2.block_hashes_exist([send1.hash()]), false);

        assert_eq!(1, node2.ledger.block_count());
        assert!(node2
            .unchecked
            .exists(&UncheckedKey::new(send2.previous(), send2.hash())));

        // Insert missing block
        node2.process_active(send1);
        assert_timely_eq(Duration::from_secs(5), || node2.ledger.block_count(), 3);
    }

    #[test]
    fn bootstrap_processor_lazy_cancel() {
        let mut system = System::new();
        let config = System::default_config_without_backlog_population();

        let mut flags = NodeFlags::new();
        flags.disable_bootstrap_bulk_push_client = true;

        let node0 = system
            .build_node()
            .config(config.clone())
            .flags(flags.clone())
            .finish();

        let mut lattice = UnsavedBlockLatticeBuilder::new();
        let key1 = PrivateKey::new();

        let send1 = lattice.genesis().send(&key1, Amount::nano(1000));
        // Start lazy bootstrap with last block in chain known
        let node1 = system.make_disconnected_node();
        establish_tcp(&node1, &node0);

        // Start "confirmed" block bootstrap
        node1
            .bootstrap_initiator
            .bootstrap_lazy(send1.hash().into(), true, "".to_owned());
        {
            node1
                .bootstrap_initiator
                .current_lazy_attempt()
                .expect("no lazy attempt found");
        }
        // Cancel failing lazy bootstrap
        assert_timely_msg(
            Duration::from_secs(20),
            || !node1.bootstrap_initiator.in_progress(),
            "attempt not cancelled",
        );
    }

    #[test]
    fn bootstrap_processor_multiple_attempts() {
        let mut system = System::new();
        let config = System::default_config_without_backlog_population();
        let mut flags = NodeFlags::new();
        flags.disable_bootstrap_bulk_push_client = true;
        let node0 = system.build_node().config(config).flags(flags).finish();

        let mut lattice = UnsavedBlockLatticeBuilder::new();
        let key1 = PrivateKey::new();
        let key2 = PrivateKey::new();
        // Generating test chain

        let send1 = lattice.genesis().send(&key1, Amount::nano(1000));
        let receive1 = lattice.account(&key1).receive(&send1);
        let send2 = lattice.account(&key1).send(&key2, Amount::nano(1000));
        let receive2 = lattice.account(&key2).receive(&send2);

        // Processing test chain
        let blocks = [send1, receive1, send2, receive2.clone()];
        node0.process_multi(&blocks);

        assert_timely_msg(
            Duration::from_secs(5),
            || node0.blocks_exist(&blocks),
            "blocks not processed",
        );

        // Start 2 concurrent bootstrap attempts
        let mut node_config = System::default_config();
        node_config.bootstrap_initiator_threads = 3;

        let node1 = system
            .build_node()
            .config(node_config)
            .disconnected()
            .finish();
        establish_tcp(&node1, &node0);
        node1
            .bootstrap_initiator
            .bootstrap_lazy(receive2.hash().into(), true, "".to_owned());
        node1
            .bootstrap_initiator
            .bootstrap(false, "".to_owned(), u32::MAX, Account::zero());

        assert_timely_msg(
            Duration::from_secs(5),
            || node1.bootstrap_initiator.current_legacy_attempt().is_some(),
            "no legacy attempt found",
        );

        // Check processed blocks
        assert_timely_msg(
            Duration::from_secs(10),
            || node1.balance(&key2.account()) > Amount::zero(),
            "balance not updated",
        );

        // Check attempts finish
        assert_timely_eq(
            Duration::from_secs(5),
            || node1.bootstrap_initiator.attempts.lock().unwrap().size(),
            0,
        );
    }

    #[test]
    fn bootstrap_processor_wallet_lazy_frontier() {
        let mut system = System::new();
        let config = System::default_config_without_backlog_population();
        let mut flags = NodeFlags::new();
        flags.disable_bootstrap_bulk_push_client = true;
        flags.disable_legacy_bootstrap = true;
        flags.disable_ascending_bootstrap = true;
        flags.disable_ongoing_bootstrap = true;
        let node0 = system.build_node().config(config).flags(flags).finish();

        let mut lattice = UnsavedBlockLatticeBuilder::new();
        let key1 = PrivateKey::new();
        let key2 = PrivateKey::new();

        // Generating test chain
        let send1 = lattice.genesis().send(&key1, Amount::nano(1000));
        let receive1 = lattice.account(&key1).receive(&send1);
        let send2 = lattice.account(&key1).send(&key2, Amount::nano(1000));
        let receive2 = lattice.account(&key2).receive(&send2);

        // Processing test chain
        let blocks = [send1, receive1, send2, receive2.clone()];
        node0.process_multi(&blocks);

        assert_timely_msg(
            Duration::from_secs(5),
            || node0.blocks_exist(&blocks),
            "blocks not processed",
        );

        // Start wallet lazy bootstrap
        let node1 = system.make_disconnected_node();
        establish_tcp(&node1, &node0);
        let wallet_id = WalletId::random();
        node1.wallets.create(wallet_id);
        node1
            .wallets
            .insert_adhoc2(&wallet_id, &key2.raw_key(), true)
            .unwrap();
        node1.bootstrap_wallet();
        {
            node1
                .bootstrap_initiator
                .current_wallet_attempt()
                .expect("no wallet attempt found");
        }
        // Check processed blocks
        assert_timely_msg(
            Duration::from_secs(10),
            || node1.block_exists(&receive2.hash()),
            "receive 2 not  found",
        )
    }

    #[test]
    fn push_diamond() {
        let mut system = System::new();
        let key = PrivateKey::new();
        let node1 = system.make_disconnected_node();
        let wallet_id = WalletId::from(100);
        node1.wallets.create(wallet_id);
        node1
            .wallets
            .insert_adhoc2(&wallet_id, &DEV_GENESIS_KEY.raw_key(), true)
            .unwrap();
        node1
            .wallets
            .insert_adhoc2(&wallet_id, &key.raw_key(), true)
            .unwrap();

        let mut lattice = UnsavedBlockLatticeBuilder::new();
        // send all balance from genesis to key
        let send1 = lattice.genesis().send(&key, Amount::MAX);
        node1.process(send1.clone()).unwrap();

        // open key account receiving all balance of genesis
        let open = lattice.account(&key).receive(&send1);
        node1.process(open.clone()).unwrap();

        // send from key to genesis 100 raw
        let send2 = lattice.account(&key).send(*DEV_GENESIS_ACCOUNT, 100);
        node1.process(send2.clone()).unwrap();

        // receive the 100 raw on genesis
        let receive = lattice.genesis().receive(&send2);
        node1.process(receive.clone()).unwrap();

        let config = System::default_config_without_backlog_population();

        let flags = NodeFlags {
            disable_ongoing_bootstrap: true,
            disable_ascending_bootstrap: true,
            ..Default::default()
        };

        let node2 = system.build_node().config(config).flags(flags).finish();
        node1
            .peer_connector
            .connect_to(node2.tcp_listener.local_address());
        assert_timely_eq(
            Duration::from_secs(5),
            || {
                node2
                    .network_info
                    .read()
                    .unwrap()
                    .count_by_mode(ChannelMode::Realtime)
            },
            1,
        );
        node1
            .bootstrap_initiator
            .bootstrap2(node2.tcp_listener.local_address(), "".to_string());
        assert_timely_eq(
            Duration::from_secs(5),
            || node2.balance(&DEV_GENESIS_ACCOUNT),
            Amount::raw(100),
        );
    }
}

mod bulk_pull {
    use super::*;
    use rsnano_core::{StateBlockArgs, UnsavedBlockLatticeBuilder};

    // If the account doesn't exist, current == end so there's no iteration
    #[test]
    fn no_address() {
        let mut system = System::new();
        let node = system.make_node();
        let bulk_pull = BulkPull {
            start: 1.into(),
            end: 2.into(),
            count: 0,
            ascending: false,
        };

        let pull_server = create_bulk_pull_server(&node, bulk_pull);

        assert_eq!(pull_server.current(), BlockHash::zero());
        assert_eq!(pull_server.request().end, BlockHash::zero());
    }

    #[test]
    fn genesis_to_end() {
        let mut system = System::new();
        let node = system.make_node();
        let bulk_pull = BulkPull {
            start: (*DEV_GENESIS_ACCOUNT).into(),
            end: BlockHash::zero(),
            count: 0,
            ascending: false,
        };

        let pull_server = create_bulk_pull_server(&node, bulk_pull);

        assert_eq!(node.latest(&DEV_GENESIS_ACCOUNT), pull_server.current());
    }

    // If we can't find the end block, send everything
    #[test]
    fn no_end() {
        let mut system = System::new();
        let node = system.make_node();
        let bulk_pull = BulkPull {
            start: (*DEV_GENESIS_ACCOUNT).into(),
            end: 1.into(),
            count: 0,
            ascending: false,
        };
        let pull_server = create_bulk_pull_server(&node, bulk_pull);
        assert_eq!(node.latest(&DEV_GENESIS_ACCOUNT), pull_server.current());
        assert_eq!(pull_server.request().end, BlockHash::zero());
    }

    #[test]
    fn end_not_owned() {
        let mut system = System::new();
        let node = system.make_node();
        let key2 = PrivateKey::new();
        let wallet_id = node.wallets.wallet_ids()[0];
        node.wallets
            .insert_adhoc2(&wallet_id, &DEV_GENESIS_KEY.raw_key(), true)
            .unwrap();
        node.wallets
            .send_action2(
                &wallet_id,
                *DEV_GENESIS_ACCOUNT,
                key2.account(),
                Amount::raw(100),
                0,
                true,
                None,
            )
            .unwrap();
        let latest = node.latest(&DEV_GENESIS_ACCOUNT);

        let open: Block = StateBlockArgs {
            key: &key2,
            previous: BlockHash::zero(),
            representative: key2.public_key(),
            balance: Amount::raw(100),
            link: latest.into(),
            work: node.work_generate_dev(&key2),
        }
        .into();

        node.process(open).unwrap();
        let bulk_pull = BulkPull {
            start: key2.account().into(),
            end: *DEV_GENESIS_HASH,
            count: 0,
            ascending: false,
        };
        let pull_server = create_bulk_pull_server(&node, bulk_pull);
        assert_eq!(pull_server.current(), pull_server.request().end);
    }

    #[test]
    fn none() {
        let mut system = System::new();
        let node = system.make_node();
        let bulk_pull = BulkPull {
            start: (*DEV_GENESIS_ACCOUNT).into(),
            end: *DEV_GENESIS_HASH,
            count: 0,
            ascending: false,
        };
        let pull_server = create_bulk_pull_server(&node, bulk_pull);
        assert_eq!(pull_server.get_next(), None);
    }

    #[test]
    fn get_next_on_open() {
        let mut system = System::new();
        let node = system.make_node();
        let bulk_pull = BulkPull {
            start: (*DEV_GENESIS_ACCOUNT).into(),
            end: 0.into(),
            count: 0,
            ascending: false,
        };
        let pull_server = create_bulk_pull_server(&node, bulk_pull);
        let block = pull_server.get_next().unwrap();
        assert!(block.previous().is_zero());
        assert_eq!(pull_server.current(), pull_server.request().end);
    }

    // Tests that the ascending flag is respected in the bulk_pull message when given a known block hash
    #[test]
    fn ascending_one_hash() {
        let mut system = System::new();
        let node = system.make_node();

        let mut lattice = UnsavedBlockLatticeBuilder::new();
        let block1 = lattice.genesis().send(*DEV_GENESIS_ACCOUNT, 100);
        node.process(block1.clone()).unwrap();

        let bulk_pull = BulkPull {
            start: (*DEV_GENESIS_HASH).into(),
            end: 0.into(),
            count: 0,
            ascending: true,
        };
        let pull_server = create_bulk_pull_server(&node, bulk_pull);
        let block_out1 = pull_server.get_next().unwrap();
        assert_eq!(block_out1.hash(), block1.hash());
        assert!(pull_server.get_next().is_none());
    }

    // Tests that the ascending flag is respected in the bulk_pull message when given an account number
    #[test]
    fn ascending_two_account() {
        let mut system = System::new();
        let node = system.make_node();

        let mut lattice = UnsavedBlockLatticeBuilder::new();
        let block1 = lattice.genesis().send(*DEV_GENESIS_ACCOUNT, 100);
        node.process(block1.clone()).unwrap();

        let bulk_pull = BulkPull {
            start: (*DEV_GENESIS_ACCOUNT).into(),
            end: 0.into(),
            count: 0,
            ascending: true,
        };
        let pull_server = create_bulk_pull_server(&node, bulk_pull);
        let block_out1 = pull_server.get_next().unwrap();
        assert_eq!(block_out1.hash(), *DEV_GENESIS_HASH);
        let block_out2 = pull_server.get_next().unwrap();
        assert_eq!(block_out2.hash(), block1.hash());
        assert!(pull_server.get_next().is_none());
    }

    // Tests that the `end' value is respected in the bulk_pull message when the ascending flag is used.
    #[test]
    fn ascending_end() {
        let mut system = System::new();
        let node = system.make_node();

        let mut lattice = UnsavedBlockLatticeBuilder::new();
        let block1 = lattice.genesis().send(*DEV_GENESIS_ACCOUNT, 100);
        node.process(block1.clone()).unwrap();

        let bulk_pull = BulkPull {
            start: (*DEV_GENESIS_ACCOUNT).into(),
            end: block1.hash(),
            count: 0,
            ascending: true,
        };
        let pull_server = create_bulk_pull_server(&node, bulk_pull);
        let block_out1 = pull_server.get_next().unwrap();
        assert_eq!(block_out1.hash(), *DEV_GENESIS_HASH);
        assert!(pull_server.get_next().is_none());
    }

    #[test]
    fn by_block() {
        let mut system = System::new();
        let node = system.make_node();

        let bulk_pull = BulkPull {
            start: (*DEV_GENESIS_HASH).into(),
            end: 0.into(),
            count: 0,
            ascending: false,
        };
        let pull_server = create_bulk_pull_server(&node, bulk_pull);
        let block_out1 = pull_server.get_next().unwrap();
        assert_eq!(block_out1.hash(), *DEV_GENESIS_HASH);
        assert!(pull_server.get_next().is_none());
    }

    #[test]
    fn by_block_single() {
        let mut system = System::new();
        let node = system.make_node();

        let bulk_pull = BulkPull {
            start: (*DEV_GENESIS_HASH).into(),
            end: *DEV_GENESIS_HASH,
            count: 0,
            ascending: false,
        };
        let pull_server = create_bulk_pull_server(&node, bulk_pull);
        let block_out1 = pull_server.get_next().unwrap();
        assert_eq!(block_out1.hash(), *DEV_GENESIS_HASH);
        assert!(pull_server.get_next().is_none());
    }

    #[test]
    fn count_limit() {
        let mut system = System::new();
        let node = system.make_node();

        let mut lattice = UnsavedBlockLatticeBuilder::new();
        let send1 = lattice
            .genesis()
            .send(*DEV_GENESIS_ACCOUNT, Amount::MAX - Amount::raw(1));
        node.process(send1.clone()).unwrap();

        let receive1 = lattice.genesis().receive(&send1);
        node.process(receive1.clone()).unwrap();

        let bulk_pull = BulkPull {
            start: receive1.hash().into(),
            end: 0.into(),
            count: 2,
            ascending: false,
        };
        let pull_server = create_bulk_pull_server(&node, bulk_pull);
        assert_eq!(pull_server.max_count(), 2);
        assert_eq!(pull_server.sent_count(), 0);

        let block = pull_server.get_next().unwrap();
        assert_eq!(receive1.hash(), block.hash());

        let block = pull_server.get_next().unwrap();
        assert_eq!(send1.hash(), block.hash());

        let block = pull_server.get_next();
        assert!(block.is_none());
    }

    fn create_bulk_pull_server(node: &Node, request: BulkPull) -> BulkPullServer {
        let response_server = create_response_server(&node);
        BulkPullServer::new(
            request,
            response_server,
            node.ledger.clone(),
            node.workers.clone(),
            node.runtime.clone(),
        )
    }
}

mod frontier_req {
    use super::*;
    use rsnano_core::UnsavedBlockLatticeBuilder;
    use rsnano_messages::FrontierReq;
    use rsnano_node::bootstrap::FrontierReqServer;
    use std::thread::sleep;

    #[test]
    fn begin() {
        let mut system = System::new();
        let node = system.make_node();

        let request = FrontierReq {
            start: Account::zero(),
            age: u32::MAX,
            count: u32::MAX,
            only_confirmed: false,
        };
        let frontier_req_server = create_frontier_req_server(&node, request);
        assert_eq!(*DEV_GENESIS_ACCOUNT, frontier_req_server.current());
        assert_eq!(*DEV_GENESIS_HASH, frontier_req_server.frontier());
    }

    #[test]
    fn end() {
        let mut system = System::new();
        let node = system.make_node();

        let request = FrontierReq {
            start: DEV_GENESIS_ACCOUNT.inc().unwrap(),
            age: u32::MAX,
            count: u32::MAX,
            only_confirmed: false,
        };
        let frontier_req_server = create_frontier_req_server(&node, request);
        assert!(frontier_req_server.current().is_zero());
    }

    #[test]
    fn count() {
        let mut system = System::new();
        let node = system.make_node();

        // Public key FB93... after genesis in accounts table
        let key1 = PrivateKey::from_hex_str(
            "ED5AE0A6505B14B67435C29FD9FEEBC26F597D147BC92F6D795FFAD7AFD3D967",
        )
        .unwrap();

        let mut lattice = UnsavedBlockLatticeBuilder::new();
        let send1 = lattice.genesis().send(&key1, Amount::nano(1000));
        node.process(send1.clone()).unwrap();

        let receive1 = lattice.account(&key1).receive(&send1);
        node.process(receive1.clone()).unwrap();

        let request = FrontierReq {
            start: Account::zero(),
            age: u32::MAX,
            count: 1,
            only_confirmed: false,
        };
        let frontier_req_server = create_frontier_req_server(&node, request);
        assert_eq!(*DEV_GENESIS_ACCOUNT, frontier_req_server.current());
        assert_eq!(send1.hash(), frontier_req_server.frontier());
    }

    #[test]
    fn time_bound() {
        let mut system = System::new();
        let node = system.make_node();

        let request = FrontierReq {
            start: Account::zero(),
            age: 1,
            count: u32::MAX,
            only_confirmed: false,
        };
        let frontier_req_server = create_frontier_req_server(&node, request.clone());
        assert_eq!(*DEV_GENESIS_ACCOUNT, frontier_req_server.current());
        // Wait 2 seconds until age of account will be > 1 seconds
        sleep(Duration::from_millis(2100));

        let frontier_req_server2 = create_frontier_req_server(&node, request.clone());
        assert_eq!(Account::zero(), frontier_req_server2.current());
    }

    #[test]
    fn time_cutoff() {
        let mut system = System::new();
        let node = system.make_node();

        let request = FrontierReq {
            start: Account::zero(),
            age: 3,
            count: u32::MAX,
            only_confirmed: false,
        };
        let frontier_req_server = create_frontier_req_server(&node, request.clone());
        assert_eq!(*DEV_GENESIS_ACCOUNT, frontier_req_server.current());
        assert_eq!(*DEV_GENESIS_HASH, frontier_req_server.frontier());
        // Wait 4 seconds until age of account will be > 3 seconds
        sleep(Duration::from_millis(4100));

        let frontier_req_server2 = create_frontier_req_server(&node, request.clone());
        assert_eq!(BlockHash::zero(), frontier_req_server2.frontier());
    }

    #[test]
    fn confirmed_frontier() {
        let mut system = System::new();
        let node = system.make_node();

        let mut key_before_genesis = PrivateKey::new();
        // Public key before genesis in accounts table
        while key_before_genesis.public_key().number() >= DEV_GENESIS_ACCOUNT.number() {
            key_before_genesis = PrivateKey::new();
        }
        let mut key_after_genesis = PrivateKey::new();
        // Public key after genesis in accounts table
        while key_after_genesis.public_key().number() <= DEV_GENESIS_ACCOUNT.number() {
            key_after_genesis = PrivateKey::new();
        }

        let mut lattice = UnsavedBlockLatticeBuilder::new();
        let send1 = lattice
            .genesis()
            .send(&key_before_genesis, Amount::nano(1000));
        node.process(send1.clone()).unwrap();

        let send2 = lattice
            .genesis()
            .send(&key_after_genesis, Amount::nano(1000));
        node.process(send2.clone()).unwrap();

        let receive1 = lattice.account(&key_before_genesis).receive(&send1);
        node.process(receive1.clone()).unwrap();

        let receive2 = lattice.account(&key_after_genesis).receive(&send2);
        node.process(receive2.clone()).unwrap();

        // Request for all accounts (confirmed only)
        let request = FrontierReq {
            start: Account::zero(),
            age: u32::MAX,
            count: u32::MAX,
            only_confirmed: true,
        };
        let frontier_req_server1 = create_frontier_req_server(&node, request.clone());
        assert_eq!(*DEV_GENESIS_ACCOUNT, frontier_req_server1.current());
        assert_eq!(*DEV_GENESIS_HASH, frontier_req_server1.frontier());

        // Request starting with account before genesis (confirmed only)
        let request2 = FrontierReq {
            start: key_before_genesis.account(),
            age: u32::MAX,
            count: u32::MAX,
            only_confirmed: true,
        };
        let frontier_req_server2 = create_frontier_req_server(&node, request2.clone());
        assert_eq!(*DEV_GENESIS_ACCOUNT, frontier_req_server2.current());
        assert_eq!(*DEV_GENESIS_HASH, frontier_req_server2.frontier());

        // Request starting with account after genesis (confirmed only)
        let request3 = FrontierReq {
            start: key_after_genesis.account(),
            age: u32::MAX,
            count: u32::MAX,
            only_confirmed: true,
        };
        let frontier_req_server3 = create_frontier_req_server(&node, request3.clone());
        assert_eq!(Account::zero(), frontier_req_server3.current());
        assert_eq!(BlockHash::zero(), frontier_req_server3.frontier());

        // Request for all accounts (unconfirmed blocks)
        let request4 = FrontierReq {
            start: Account::zero(),
            age: u32::MAX,
            count: u32::MAX,
            only_confirmed: false,
        };
        let frontier_req_server4 = create_frontier_req_server(&node, request4.clone());
        assert_eq!(key_before_genesis.account(), frontier_req_server4.current());
        assert_eq!(receive1.hash(), frontier_req_server4.frontier());

        // Request starting with account after genesis (unconfirmed blocks)
        let request5 = FrontierReq {
            start: key_after_genesis.account(),
            age: u32::MAX,
            count: u32::MAX,
            only_confirmed: false,
        };
        let frontier_req_server5 = create_frontier_req_server(&node, request5.clone());
        assert_eq!(key_after_genesis.account(), frontier_req_server5.current());
        assert_eq!(receive2.hash(), frontier_req_server5.frontier());

        // Confirm account before genesis (confirmed only)
        node.confirm(receive1.hash());
        let request6 = FrontierReq {
            start: key_before_genesis.account(),
            age: u32::MAX,
            count: u32::MAX,
            only_confirmed: true,
        };
        let frontier_req_server6 = create_frontier_req_server(&node, request6.clone());
        assert_eq!(key_before_genesis.account(), frontier_req_server6.current());
        assert_eq!(receive1.hash(), frontier_req_server6.frontier());

        // Confirm account after genesis (confirmed only)
        node.confirm(receive2.hash());
        let request7 = FrontierReq {
            start: key_after_genesis.account(),
            age: u32::MAX,
            count: u32::MAX,
            only_confirmed: true,
        };
        let frontier_req_server7 = create_frontier_req_server(&node, request7.clone());
        assert_eq!(key_after_genesis.account(), frontier_req_server7.current());
        assert_eq!(receive2.hash(), frontier_req_server7.frontier());
    }

    fn create_frontier_req_server(node: &Node, request: FrontierReq) -> FrontierReqServer {
        let response_server = create_response_server(&node);
        FrontierReqServer::new(
            response_server,
            request,
            node.workers.clone(),
            node.ledger.clone(),
            node.runtime.clone(),
        )
    }
}

mod bulk_pull_account {
    use super::*;
    use rsnano_messages::{BulkPullAccount, BulkPullAccountFlags};
    use rsnano_node::bootstrap::BulkPullAccountServer;

    #[test]
    fn basic() {
        let mut system = System::new();
        let mut config = System::default_config();
        config.receive_minimum = Amount::raw(20);
        let node = system.build_node().config(config).finish();
        let key1 = PrivateKey::new();
        let wallet_id = node.wallets.wallet_ids()[0];
        node.wallets
            .insert_adhoc2(&wallet_id, &DEV_GENESIS_KEY.raw_key(), true)
            .unwrap();
        node.wallets
            .insert_adhoc2(&wallet_id, &key1.raw_key(), true)
            .unwrap();
        let _send1 = node
            .wallets
            .send_action2(
                &wallet_id,
                *DEV_GENESIS_ACCOUNT,
                key1.account(),
                Amount::raw(25),
                0,
                true,
                None,
            )
            .unwrap();
        let send2 = node
            .wallets
            .send_action2(
                &wallet_id,
                *DEV_GENESIS_ACCOUNT,
                key1.account(),
                Amount::raw(10),
                0,
                true,
                None,
            )
            .unwrap();
        let _send3 = node
            .wallets
            .send_action2(
                &wallet_id,
                *DEV_GENESIS_ACCOUNT,
                key1.account(),
                Amount::raw(2),
                0,
                true,
                None,
            )
            .unwrap();
        assert_timely_eq(
            Duration::from_secs(5),
            || node.balance(&key1.account()),
            Amount::raw(25),
        );
        let response_server = create_response_server(&node);
        {
            let payload = BulkPullAccount {
                account: key1.account(),
                minimum_amount: Amount::raw(5),
                flags: BulkPullAccountFlags::PendingHashAndAmount,
            };

            let pull_server = BulkPullAccountServer::new(
                response_server.clone(),
                payload,
                node.workers.clone(),
                node.ledger.clone(),
                node.runtime.clone(),
            );

            assert_eq!(pull_server.invalid_request(), false);
            assert_eq!(pull_server.pending_include_address(), false);
            assert_eq!(pull_server.pending_address_only(), false);
            assert_eq!(pull_server.current_key().receiving_account, key1.account());
            assert_eq!(pull_server.current_key().send_block_hash, BlockHash::zero());
            let (key, info) = pull_server.get_next().unwrap();
            assert_eq!(key.send_block_hash, send2.hash());
            assert_eq!(info.amount, Amount::raw(10));
            assert_eq!(info.source, *DEV_GENESIS_ACCOUNT);
            assert!(pull_server.get_next().is_none())
        }

        {
            let payload = BulkPullAccount {
                account: key1.account(),
                minimum_amount: Amount::zero(),
                flags: BulkPullAccountFlags::PendingAddressOnly,
            };

            let pull_server = BulkPullAccountServer::new(
                response_server,
                payload,
                node.workers.clone(),
                node.ledger.clone(),
                node.runtime.clone(),
            );

            assert_eq!(pull_server.pending_address_only(), true);
            let (_key, info) = pull_server.get_next().unwrap();
            assert_eq!(info.source, *DEV_GENESIS_ACCOUNT);
            assert!(pull_server.get_next().is_none());
        }
    }
}

#[test]
fn bulk_genesis() {
    let mut system = System::new();
    let config = System::default_config_without_backlog_population();
    let flags = NodeFlags {
        disable_bootstrap_bulk_push_client: true,
        disable_lazy_bootstrap: true,
        ..Default::default()
    };
    let node1 = system.build_node().config(config).flags(flags).finish();
    node1.insert_into_wallet(&DEV_GENESIS_KEY);

    let node2 = system.make_disconnected_node();
    let latest1 = node1.latest(&DEV_GENESIS_ACCOUNT);
    let latest2 = node2.latest(&DEV_GENESIS_ACCOUNT);
    assert_eq!(latest1, latest2);
    let key2 = PrivateKey::new();
    let wallet_id = node1.wallets.wallet_ids()[0];
    let _send = node1
        .wallets
        .send_action2(
            &wallet_id,
            *DEV_GENESIS_ACCOUNT,
            key2.public_key().as_account(),
            Amount::raw(100),
            0,
            true,
            None,
        )
        .unwrap();
    let latest3 = node1.latest(&DEV_GENESIS_ACCOUNT);
    assert_ne!(latest1, latest3);

    node2
        .peer_connector
        .connect_to(node1.tcp_listener.local_address());
    node2
        .bootstrap_initiator
        .bootstrap2(node1.tcp_listener.local_address(), "".into());
    assert_timely(Duration::from_secs(10), || {
        node2.latest(&DEV_GENESIS_ACCOUNT) == node1.latest(&DEV_GENESIS_ACCOUNT)
    });
    assert_eq!(
        node2.latest(&DEV_GENESIS_ACCOUNT),
        node1.latest(&DEV_GENESIS_ACCOUNT)
    );
}

#[test]
#[ignore = "This test fails a lot"]
fn bulk_offline_send() {
    let mut system = System::new();
    let config = System::default_config_without_backlog_population();
    let flags = NodeFlags {
        disable_bootstrap_bulk_push_client: true,
        disable_lazy_bootstrap: true,
        ..Default::default()
    };
    let node1 = system.build_node().config(config).flags(flags).finish();
    node1.insert_into_wallet(&DEV_GENESIS_KEY);
    let amount = node1.config.receive_minimum;
    let node2 = system.make_disconnected_node();
    let key2 = PrivateKey::new();
    let wallet_id2 = WalletId::random();
    node2.wallets.create(wallet_id2);
    node2
        .wallets
        .insert_adhoc2(&wallet_id2, &key2.raw_key(), true)
        .unwrap();

    // send amount from genesis to key2, it will be autoreceived
    let wallet_id1 = node1.wallets.wallet_ids()[0];
    let send1 = node1
        .wallets
        .send_action2(
            &wallet_id1,
            *DEV_GENESIS_ACCOUNT,
            key2.public_key().into(),
            node1.config.receive_minimum,
            0,
            true,
            None,
        )
        .unwrap();

    // Wait to finish election background tasks
    assert_timely_eq(Duration::from_secs(5), || node1.active.len(), 0);
    assert_timely(Duration::from_secs(5), || {
        node1.block_confirmed(&send1.hash())
    });
    assert_eq!(Amount::MAX - amount, node1.balance(&DEV_GENESIS_ACCOUNT));

    // Initiate bootstrap
    node2
        .peer_connector
        .connect_to(node1.tcp_listener.local_address());
    node2
        .bootstrap_initiator
        .bootstrap2(node1.tcp_listener.local_address(), "".into());

    // Nodes should find each other after bootstrap initiation
    assert_timely(Duration::from_secs(5), || {
        !node1.network_info.read().unwrap().len() > 0
    });
    assert_timely(Duration::from_secs(5), || {
        !node2.network_info.read().unwrap().len() > 0
    });

    // Send block arrival via bootstrap
    assert_timely_eq(
        Duration::from_secs(5),
        || node2.balance(&DEV_GENESIS_ACCOUNT),
        Amount::MAX - amount,
    );
    // Receiving send block
    assert_timely_eq(
        Duration::from_secs(5),
        || node2.balance(&key2.public_key().into()),
        amount,
    );
}

#[test]
#[ignore = "This test fails a lot"]
fn bulk_genesis_pruning() {
    let mut system = System::new();
    let config = NodeConfig {
        enable_voting: false,
        ..System::default_config_without_backlog_population()
    };
    let mut flags = NodeFlags {
        disable_bootstrap_bulk_push_client: true,
        disable_lazy_bootstrap: true,
        disable_ongoing_bootstrap: true,
        disable_ascending_bootstrap: true,
        enable_pruning: true,
        ..Default::default()
    };
    let node1 = system
        .build_node()
        .config(config)
        .flags(flags.clone())
        .finish();
    let blocks = setup_chain(&node1, 3, &DEV_GENESIS_KEY, true);
    let send1 = &blocks[0];
    let send2 = &blocks[1];
    let send3 = &blocks[2];
    assert_eq!(4, node1.ledger.block_count());

    node1.ledger_pruning(2, false);
    assert_eq!(2, node1.ledger.pruned_count());
    assert_eq!(4, node1.ledger.block_count());
    assert!(node1
        .ledger
        .store
        .pruned
        .exists(&node1.ledger.read_txn(), &send1.hash()));
    assert_eq!(node1.block_exists(&send1.hash()), false);
    assert!(node1
        .ledger
        .store
        .pruned
        .exists(&node1.ledger.read_txn(), &send2.hash()));
    assert_eq!(node1.block_exists(&send2.hash()), false);
    assert_eq!(node1.block_exists(&send3.hash()), true);

    // Bootstrap with missing blocks for node2
    flags.enable_pruning = false;
    let node2 = system.build_node().flags(flags).disconnected().finish();
    node2
        .peer_connector
        .connect_to(node1.tcp_listener.local_address());
    node2
        .bootstrap_initiator
        .bootstrap2(node1.tcp_listener.local_address(), "".into());
    assert_timely(Duration::from_secs(5), || {
        node2
            .stats
            .count(StatType::Bootstrap, DetailType::Initiate, Direction::Out)
            >= 1
    });
    assert_timely(Duration::from_secs(5), || {
        !node2.bootstrap_initiator.in_progress()
    });

    // node2 still missing blocks
    assert_eq!(1, node2.ledger.block_count());
    {
        let _tx = node2.ledger.rw_txn();
        node2.unchecked.clear();
    }

    // Insert pruned blocks
    node2.process_active(send1.clone());
    node2.process_active(send2.clone());
    assert_timely_eq(Duration::from_secs(5), || node2.ledger.block_count(), 3);

    // New bootstrap to sync up everything
    assert_timely_eq(
        Duration::from_secs(5),
        || {
            node2
                .bootstrap_initiator
                .connections
                .connections_count
                .load(Ordering::SeqCst)
        },
        0,
    );
    node2
        .peer_connector
        .connect_to(node1.tcp_listener.local_address());
    node2
        .bootstrap_initiator
        .bootstrap2(node1.tcp_listener.local_address(), "".into());
    assert_timely(Duration::from_secs(5), || {
        node2.latest(&DEV_GENESIS_ACCOUNT) == node1.latest(&DEV_GENESIS_ACCOUNT)
    });
}

#[test]
#[ignore = "always fails?!"]
// TODO Gustav: I've disabled this test because it fails I haven't found out why yet.
// Legacy bootstrap will be removed soon and pruning is no priority currently
fn push_diamond_pruning() {
    let mut system = System::new();

    let node0 = system
        .build_node()
        .config(System::default_config_without_backlog_population())
        .flags(NodeFlags {
            disable_ascending_bootstrap: true,
            disable_ongoing_bootstrap: true,
            ..Default::default()
        })
        .finish();

    let node1 = system
        .build_node()
        .config(NodeConfig {
            enable_voting: false,
            ..System::default_config_without_backlog_population()
        })
        .flags(NodeFlags {
            enable_pruning: true,
            ..Default::default()
        })
        .finish();

    let mut lattice = UnsavedBlockLatticeBuilder::new();
    let key = PrivateKey::new();

    // send all balance from genesis to key
    let send1 = lattice.genesis().send(&key, Amount::MAX);
    node1.process(send1.clone()).unwrap();

    // receive all balance on key
    let open = lattice.account(&key).receive(&send1);
    node1.process(open.clone()).unwrap();

    // 1st bootstrap
    node1
        .bootstrap_initiator
        .bootstrap2(node0.tcp_listener.local_address(), "".to_owned());
    assert_timely_eq(
        Duration::from_secs(5),
        || node0.balance(&key.public_key().as_account()),
        Amount::MAX,
    );
    assert_timely_eq(
        Duration::from_secs(5),
        || node1.balance(&key.public_key().as_account()),
        Amount::MAX,
    );

    // Process more blocks & prune old

    // send 100 raw from key to genesis
    let send2 = lattice.account(&key).send(*DEV_GENESIS_ACCOUNT, 100);
    node1.process(send2.clone()).unwrap();

    // receive the 100 raw from key on genesis
    let receive = lattice.genesis().receive(&send2);
    node1.process(receive.clone()).unwrap();

    {
        let mut tx = node1.ledger.rw_txn();
        node1.ledger.confirm(&mut tx, open.hash());
        assert_eq!(node1.ledger.pruning_action(&mut tx, &send1.hash(), 2), 1);
        assert_eq!(node1.ledger.pruning_action(&mut tx, &open.hash(), 1), 1);
        assert_eq!(node1.ledger.pruned_count(), 2);
    }

    // 2nd bootstrap
    node1
        .bootstrap_initiator
        .bootstrap2(node0.tcp_listener.local_address(), "".to_owned());
    assert_timely_eq(
        Duration::from_secs(5),
        || node0.balance(&DEV_GENESIS_ACCOUNT),
        Amount::raw(100),
    );
    assert_timely_eq(
        Duration::from_secs(5),
        || node1.balance(&DEV_GENESIS_ACCOUNT),
        Amount::raw(100),
    );
}

#[test]
fn push_one() {
    let mut system = System::new();

    let node0 = system
        .build_node()
        .config(System::default_config_without_backlog_population())
        .finish();

    let key1 = PrivateKey::new();
    let node1 = system.make_disconnected_node();
    node1.insert_into_wallet(&DEV_GENESIS_KEY);

    // send 100 raw from genesis to key1
    node1
        .wallets
        .send_action2(
            &node1.wallets.wallet_ids()[0],
            *DEV_GENESIS_ACCOUNT,
            key1.public_key().as_account(),
            Amount::raw(100),
            0,
            true,
            None,
        )
        .unwrap();

    assert_timely_eq(
        Duration::from_secs(5),
        || node1.balance(&DEV_GENESIS_ACCOUNT),
        Amount::MAX - Amount::raw(100),
    );

    node1
        .peer_connector
        .connect_to(node0.tcp_listener.local_address());
    node1
        .bootstrap_initiator
        .bootstrap2(node0.tcp_listener.local_address(), "".to_string());
    assert_timely_eq(
        Duration::from_secs(5),
        || node0.balance(&DEV_GENESIS_ACCOUNT),
        Amount::MAX - Amount::raw(100),
    );
}

#[test]
fn lazy_max_pull_count() {
    let mut system = System::new();
    let node0 = system
        .build_node()
        .config(System::default_config_without_backlog_population())
        .flags(NodeFlags {
            disable_bootstrap_bulk_push_client: true,
            ..Default::default()
        })
        .finish();

    let mut lattice = UnsavedBlockLatticeBuilder::new();
    let key1 = PrivateKey::new();
    let key2 = PrivateKey::new();

    // Generating test chain
    let send1 = lattice.genesis().send(&key1, 1000);
    let receive1 = lattice.account(&key1).receive(&send1);
    let send2 = lattice.account(&key1).send(&key2, 1000);
    let receive2 = lattice.account(&key2).receive(&send2);
    let change1 = lattice.account(&key2).change(&key1);
    let change2 = lattice.account(&key2).change(&*DEV_GENESIS_KEY);
    let change3 = lattice.account(&key2).change(&key2);

    // Processing test chain
    node0.process_multi(&[
        send1,
        receive1,
        send2,
        receive2,
        change1,
        change2,
        change3.clone(),
    ]);

    // Start lazy bootstrap with last block in chain known
    let node1 = system.make_node();
    node1
        .bootstrap_initiator
        .bootstrap_lazy(change3.hash().into(), false, "".to_string());

    // Check processed blocks
    assert_timely(Duration::from_secs(10), || {
        node1.block_exists(&change3.hash())
    });
}

fn create_response_server(node: &Node) -> Arc<ResponseServer> {
    let channel = Channel::create(
        Arc::new(ChannelInfo::new_test_instance()),
        TcpStream::new_null(),
        Arc::new(BandwidthLimiter::default()),
        node.steady_clock.clone(),
        Arc::new(NullNetworkObserver::new()),
        &node.runtime,
    );

    Arc::new(ResponseServer::new(
        node.network_info.clone(),
        node.inbound_message_queue.clone(),
        channel,
        node.network_filter.clone(),
        Arc::new(node.network_params.clone()),
        node.stats.clone(),
        true,
        node.syn_cookies.clone(),
        node.node_id.clone(),
        node.runtime.clone(),
        node.ledger.clone(),
        node.workers.clone(),
        node.block_processor.clone(),
        node.bootstrap_initiator.clone(),
        node.flags.clone(),
        Arc::new(Mutex::new(LatestKeepalives::default())),
    ))
}
