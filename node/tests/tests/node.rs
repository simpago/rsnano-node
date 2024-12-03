use rsnano_core::{
    utils::milliseconds_since_epoch, work::WorkPool, Account, Amount, Block, BlockBase,
    BlockBuilder, BlockHash, DifficultyV1, Epoch, LegacySendBlockBuilder, Link, OpenBlock,
    PrivateKey, PublicKey, QualifiedRoot, Root, SendBlock, Signature, StateBlock, UncheckedInfo,
    Vote, VoteSource, VoteWithWeightInfo, DEV_GENESIS_KEY,
};
use rsnano_ledger::{
    BlockStatus, Writer, DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH, DEV_GENESIS_PUB_KEY,
};
use rsnano_messages::{ConfirmAck, Message, Publish};
use rsnano_network::{ChannelId, DropPolicy, TrafficType};
use rsnano_node::{
    block_processing::BlockSource,
    bootstrap::BootstrapInitiatorExt,
    config::NodeFlags,
    consensus::{ActiveElectionsExt, VoteApplierExt},
    stats::{DetailType, Direction, StatType},
    wallets::WalletsExt,
};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread::sleep,
    time::Duration,
};
use test_helpers::{
    activate_hashes, assert_always_eq, assert_never, assert_timely, assert_timely_eq,
    assert_timely_msg, establish_tcp, get_available_port, make_fake_channel, start_election,
    start_elections, System,
};

#[test]
fn pruning_depth_max_depth() {
    let mut system = System::new();

    let mut node_config = System::default_config();
    node_config.enable_voting = false; // Pruning and voting are incompatible in this test
    node_config.max_pruning_depth = 1; // Pruning with max depth 1

    let mut node_flags = NodeFlags::default();
    node_flags.enable_pruning = true;

    let node1 = system
        .build_node()
        .config(node_config)
        .flags(node_flags)
        .finish();
    let key1 = PrivateKey::new();

    // Create the first send block
    let latest_hash = *DEV_GENESIS_HASH;
    let send1 = LegacySendBlockBuilder::new()
        .previous(latest_hash)
        .destination(key1.account())
        .balance(Amount::MAX - Amount::nano(1000))
        .sign(DEV_GENESIS_KEY.clone())
        .work(node1.work_generate_dev(latest_hash))
        .build();

    // Process the first send block
    node1.process_active(send1.clone().into());

    // Create the second send block
    let latest_hash = send1.hash();
    let send2 = LegacySendBlockBuilder::new()
        .previous(latest_hash)
        .destination(key1.account())
        .balance(Amount::raw(0))
        .sign(DEV_GENESIS_KEY.clone())
        .work(node1.work_generate_dev(latest_hash))
        .build();

    // Process the second send block
    node1.process_active(send2.clone().into());

    // Confirm both blocks
    node1.confirm(send1.hash().clone());
    assert_timely(Duration::from_secs(5), || {
        node1.block_confirmed(&send1.hash())
    });

    node1.confirm(send2.hash().clone());
    assert_timely(Duration::from_secs(5), || {
        node1.block_confirmed(&send2.hash())
    });

    // Perform pruning
    node1.ledger_pruning(1, true);

    // Check the pruning result
    assert_eq!(node1.ledger.pruned_count(), 1);
    assert_eq!(node1.ledger.block_count(), 3);

    let tx = node1.ledger.read_txn();

    // Ensure that the genesis block, send1, and send2 either exist or are pruned
    assert!(node1
        .ledger
        .any()
        .block_exists_or_pruned(&tx, &*DEV_GENESIS_HASH));
    assert!(node1
        .ledger
        .any()
        .block_exists_or_pruned(&tx, &send1.hash()));
    assert!(node1
        .ledger
        .any()
        .block_exists_or_pruned(&tx, &send2.hash()));
}

// Test that a node configured with `enable_pruning` and `max_pruning_age = 1s` will automatically
// prune old confirmed blocks without explicitly saying `node.ledger_pruning` in the unit test
#[test]
fn pruning_automatic() {
    let mut system = System::new();

    let mut node_config = System::default_config();
    node_config.enable_voting = false; // Pruning and voting are incompatible
    node_config.max_pruning_age_s = 1;

    let mut node_flags = NodeFlags::default();
    node_flags.enable_pruning = true;

    let node1 = system
        .build_node()
        .config(node_config)
        .flags(node_flags)
        .finish();
    let key1 = PrivateKey::new();

    let latest_hash = *DEV_GENESIS_HASH;
    let send1 = LegacySendBlockBuilder::new()
        .previous(latest_hash)
        .destination(key1.account())
        .balance(Amount::MAX - Amount::nano(1000))
        .sign(DEV_GENESIS_KEY.clone())
        .work(node1.work_generate_dev(latest_hash))
        .build();

    node1.process_active(send1.clone().into());

    let latest_hash = send1.hash();
    let send2 = LegacySendBlockBuilder::new()
        .previous(latest_hash)
        .destination(key1.account())
        .balance(Amount::raw(0))
        .sign(DEV_GENESIS_KEY.clone())
        .work(node1.work_generate_dev(latest_hash))
        .build();

    node1.process_active(send2.clone().into());

    assert_timely(Duration::from_secs(5), || node1.block_exists(&send2.hash()));

    node1.confirm(send1.hash().clone());
    assert_timely(Duration::from_secs(5), || {
        node1.block_confirmed(&send1.hash())
    });

    node1.confirm(send2.hash().clone());
    assert_timely(Duration::from_secs(5), || {
        node1.block_confirmed(&send2.hash())
    });

    assert_eq!(node1.ledger.block_count(), 3);

    assert_timely_eq(Duration::from_secs(5), || node1.ledger.pruned_count(), 1);

    assert_eq!(node1.ledger.pruned_count(), 1);
    assert_eq!(node1.ledger.block_count(), 3);

    let tx = node1.ledger.read_txn();

    assert!(node1
        .ledger
        .any()
        .block_exists_or_pruned(&tx, &*DEV_GENESIS_HASH));
    assert!(node1
        .ledger
        .any()
        .block_exists_or_pruned(&tx, &send1.hash()));
    assert!(node1
        .ledger
        .any()
        .block_exists_or_pruned(&tx, &send2.hash()));
}

#[test]
fn deferred_dependent_elections() {
    let mut system = System::new();

    let node_config_1 = System::default_config_without_backlog_population();
    let node_config_2 = System::default_config_without_backlog_population();

    let mut node_flags = NodeFlags::default();
    node_flags.disable_request_loop = true;
    let node1 = system
        .build_node()
        .config(node_config_1)
        .flags(node_flags.clone())
        .finish();
    let node2 = system
        .build_node()
        .config(node_config_2)
        .flags(node_flags)
        .finish();

    let key = PrivateKey::new();

    let send1 = BlockBuilder::state()
        .account(*DEV_GENESIS_ACCOUNT)
        .previous(*DEV_GENESIS_HASH)
        .representative(*DEV_GENESIS_ACCOUNT)
        .link(key.account())
        .balance(Amount::MAX - Amount::raw(1))
        .sign(&DEV_GENESIS_KEY)
        .work(node1.work_generate_dev(*DEV_GENESIS_HASH))
        .build();

    let open = BlockBuilder::state()
        .account(key.public_key())
        .previous(BlockHash::zero())
        .representative(key.public_key())
        .link(send1.hash())
        .balance(Amount::raw(1))
        .sign(&key)
        .work(node1.work_generate_dev(&key))
        .build();

    let send1_state_block = match &send1 {
        Block::State(state_block) => state_block,
        _ => panic!("Expected a StateBlock"),
    };

    let send2 = BlockBuilder::state()
        .account(*DEV_GENESIS_ACCOUNT)
        .from(&send1_state_block)
        .previous(send1.hash())
        .balance(send1.balance_field().unwrap() - Amount::raw(1))
        .link(key.account())
        .sign(&DEV_GENESIS_KEY)
        .work(node1.work_generate_dev(send1.hash()))
        .build();

    let open_state_block = match &open {
        Block::State(state_block) => state_block,
        _ => panic!("Expected a StateBlock"),
    };

    let receive = BlockBuilder::state()
        .account(key.account())
        .from(&open_state_block)
        .previous(open.hash())
        .link(send2.hash())
        .balance(Amount::raw(2))
        .sign(&key)
        .work(node1.work_generate_dev(open.hash()))
        .build();

    let receive_state_block = match &receive {
        Block::State(state_block) => state_block,
        _ => panic!("Expected a StateBlock"),
    };

    let fork = BlockBuilder::state()
        .account(key.account())
        .from(&receive_state_block)
        .representative(*DEV_GENESIS_ACCOUNT)
        .sign(&key)
        .build();

    node1.process_local(send1.clone().into()).unwrap();
    let election_send1 = start_election(&node1, &send1.hash());

    node1.process_local(open.clone().into()).unwrap();
    node1.process_local(send2.clone().into()).unwrap();

    assert_timely(std::time::Duration::from_secs(5), || {
        node1.block_exists(&open.hash())
    });
    assert_timely(std::time::Duration::from_secs(5), || {
        node1.block_exists(&send2.hash())
    });

    assert_never(std::time::Duration::from_millis(500), || {
        node1.active.election(&open.qualified_root()).is_some()
            || node1.active.election(&send2.qualified_root()).is_some()
    });

    assert_timely(std::time::Duration::from_secs(5), || {
        node2.block_exists(&open.hash())
    });
    assert_timely(std::time::Duration::from_secs(5), || {
        node2.block_exists(&send2.hash())
    });

    node1.process_local(open.clone().into()).unwrap();

    assert_never(std::time::Duration::from_millis(500), || {
        node1.active.election(&open.qualified_root()).is_some()
    });

    start_election(&node1, &open.hash());
    node1.active.erase(&open.qualified_root());
    assert!(node1.active.election(&open.qualified_root()).is_none());

    node1.process_local(open.clone().into()).unwrap();

    assert_never(std::time::Duration::from_millis(500), || {
        node1.active.election(&open.qualified_root()).is_some()
    });

    node1.active.erase(&open.qualified_root());
    node1.active.erase(&send2.qualified_root());
    assert!(!node1.active.election(&open.qualified_root()).is_some());
    assert!(!node1.active.election(&send2.qualified_root()).is_some());

    node1.active.force_confirm(&election_send1);
    assert_timely(std::time::Duration::from_secs(5), || {
        node1.block_confirmed(&send1.hash())
    });
    assert_timely(std::time::Duration::from_secs(5), || {
        node1.active.election(&open.qualified_root()).is_some()
    });
    assert_timely(std::time::Duration::from_secs(5), || {
        node1.active.election(&send2.qualified_root()).is_some()
    });

    let election_open = node1.active.election(&open.qualified_root());
    assert!(election_open.is_some());

    let election_send2 = node1.active.election(&send2.qualified_root()).unwrap();

    node1.process_local(receive.clone().into()).unwrap();
    assert!(node1.active.election(&receive.qualified_root()).is_none());

    node1.active.force_confirm(election_open.as_ref().unwrap());
    assert_timely(std::time::Duration::from_secs(5), || {
        node1.block_confirmed(&open.hash())
    });
    assert!(!node1
        .ledger
        .dependents_confirmed_for_unsaved_block(&node1.store.tx_begin_read(), &receive));

    assert_never(std::time::Duration::from_millis(500), || {
        node1.active.election(&receive.qualified_root()).is_some()
    });

    node1
        .ledger
        .rollback(&mut node1.store.tx_begin_write(), &receive.hash())
        .unwrap();
    assert!(!node1.block_exists(&receive.hash()));

    node1.process_local(receive.clone().into()).unwrap();
    assert_timely(std::time::Duration::from_secs(5), || {
        node1.block_exists(&receive.hash())
    });

    assert_never(std::time::Duration::from_millis(500), || {
        node1.active.election(&receive.qualified_root()).is_some()
    });

    assert_eq!(
        BlockStatus::Fork,
        node1.process_local(fork.clone().into()).unwrap()
    );

    node1.process_local(fork.clone().into()).unwrap();
    assert_never(std::time::Duration::from_millis(500), || {
        node1.active.election(&receive.qualified_root()).is_some()
    });

    node1.active.force_confirm(&election_send2);
    assert_timely(std::time::Duration::from_secs(5), || {
        node1.block_confirmed(&send2.hash())
    });
    assert_timely(std::time::Duration::from_secs(5), || {
        node1.active.election(&receive.qualified_root()).is_some()
    });
}

#[test]
fn rollback_gap_source() {
    let mut system = System::new();
    let mut node_config = System::default_config();
    node_config.peering_port = Some(get_available_port());
    let node = system.build_node().config(node_config).finish();

    let key = PrivateKey::new();

    let send1 = BlockBuilder::state()
        .account(*DEV_GENESIS_ACCOUNT)
        .previous(*DEV_GENESIS_HASH)
        .representative(*DEV_GENESIS_ACCOUNT)
        .link(key.account())
        .balance(Amount::MAX - Amount::raw(1))
        .sign(&DEV_GENESIS_KEY)
        .work(node.work_generate_dev(*DEV_GENESIS_HASH))
        .build();

    let fork1a = BlockBuilder::state()
        .account(key.public_key())
        .previous(BlockHash::zero())
        .representative(key.public_key())
        .link(send1.hash())
        .balance(Amount::raw(1))
        .sign(&key)
        .work(node.work_generate_dev(&key))
        .build();

    let send1_state_block = match &send1 {
        Block::State(state_block) => state_block,
        _ => panic!("Expected a StateBlock"),
    };

    let send2 = BlockBuilder::state()
        .account(*DEV_GENESIS_ACCOUNT)
        .from(&send1_state_block)
        .previous(send1.hash())
        .balance(send1.balance_field().unwrap() - Amount::raw(1))
        .link(key.account())
        .sign(&DEV_GENESIS_KEY)
        .work(node.work_generate_dev(send1.hash()))
        .build();

    let fork1a_state_block = match &fork1a {
        Block::State(state_block) => state_block,
        _ => panic!("Expected a StateBlock"),
    };

    let fork1b = BlockBuilder::state()
        .account(key.account())
        .from(&fork1a_state_block)
        .link(send2.hash())
        .sign(&key)
        .build();

    assert_eq!(
        BlockStatus::Progress,
        node.process_local(send1.clone()).unwrap()
    );
    assert_eq!(
        BlockStatus::Progress,
        node.process_local(fork1a.clone()).unwrap()
    );

    assert!(!node.block_exists(&send2.hash()));
    node.block_processor.force(fork1b.clone());

    assert_timely_eq(
        Duration::from_secs(5),
        || node.block(&fork1a.hash()).is_none(),
        true,
    );

    assert_timely_eq(
        Duration::from_secs(5),
        || {
            node.stats
                .count(StatType::Rollback, DetailType::Open, Direction::In)
        },
        1,
    );

    assert!(!node.block_exists(&fork1b.hash()));

    node.process_active(fork1a.clone());
    assert_timely_eq(
        Duration::from_secs(5),
        || node.block_exists(&fork1a.hash()),
        true,
    );

    assert_eq!(
        BlockStatus::Progress,
        node.process_local(send2.clone()).unwrap()
    );
    node.block_processor.force(fork1b.clone());

    assert_timely_eq(
        Duration::from_secs(5),
        || {
            node.stats
                .count(StatType::Rollback, DetailType::Open, Direction::In)
        },
        2,
    );

    assert_timely_eq(
        Duration::from_secs(5),
        || node.block_exists(&fork1b.hash()),
        true,
    );

    assert!(!node.block_exists(&fork1a.hash()));
}

#[test]
fn vote_by_hash_bundle() {
    // Initialize the test system with one node
    let mut system = System::new();
    let node = system.make_node();
    let wallet_id = node.wallets.wallet_ids()[0];

    // Prepare a vector to hold the blocks
    let mut blocks = Vec::new();

    // Create the first block in the chain
    let block = BlockBuilder::state()
        .account(*DEV_GENESIS_ACCOUNT)
        .previous(*DEV_GENESIS_HASH)
        .representative(*DEV_GENESIS_ACCOUNT)
        .balance(Amount::MAX - Amount::raw(1))
        .link(*DEV_GENESIS_ACCOUNT)
        .sign(&DEV_GENESIS_KEY)
        .work(node.work_generate_dev(*DEV_GENESIS_HASH))
        .build();

    blocks.push(block.clone());
    assert_eq!(BlockStatus::Progress, node.process_local(block).unwrap());

    // Create a chain of blocks
    for i in 2..10 {
        let prev_block = match blocks.last().unwrap() {
            Block::State(state_block) => state_block,
            _ => panic!("Expected a StateBlock"),
        };
        let hash: BlockHash = prev_block.hash();
        let block = BlockBuilder::state()
            .from(prev_block)
            .previous(hash)
            .balance(Amount::MAX - Amount::raw(i))
            .sign(&DEV_GENESIS_KEY)
            .work(node.work_generate_dev(hash))
            .build();
        blocks.push(block.clone());
        assert_eq!(BlockStatus::Progress, node.process_local(block).unwrap());
    }

    // Confirm the last block to confirm the entire chain
    node.ledger
        .confirm(&mut node.ledger.rw_txn(), blocks.last().unwrap().hash());

    // Insert the genesis key and a new key into the wallet
    node.wallets
        .insert_adhoc2(&wallet_id, &DEV_GENESIS_KEY.private_key(), true)
        .unwrap();
    let key1 = PrivateKey::new();
    node.wallets
        .insert_adhoc2(&wallet_id, &key1.private_key(), true)
        .unwrap();

    // Set up an observer to track the maximum number of hashes in a vote
    let max_hashes = Arc::new(AtomicUsize::new(0));
    let max_hashes_clone = Arc::clone(&max_hashes);

    node.vote_router.add_vote_processed_observer(Box::new(
        move |vote: &Arc<Vote>, _vote_source, _vote_code| {
            let hashes_size = vote.hashes.len();
            let current_max = max_hashes_clone.load(Ordering::Relaxed);
            if hashes_size > current_max {
                max_hashes_clone.store(hashes_size, Ordering::Relaxed);
            }
        },
    ));

    // Enqueue vote requests for all the blocks
    for block in &blocks {
        node.vote_generators
            .generate_non_final_vote(&block.root(), &block.hash());
    }

    // Verify that bundling occurs
    assert_timely(Duration::from_secs(20), || {
        max_hashes.load(Ordering::Relaxed) >= 3
    });
}

#[test]
fn confirm_quorum() {
    let mut system = System::new();
    let node1 = system.make_node();
    let wallet_id = node1.wallets.wallet_ids()[0];
    node1
        .wallets
        .insert_adhoc2(&wallet_id, &DEV_GENESIS_KEY.private_key(), true)
        .unwrap();

    // Put greater than node.delta() in pending so quorum can't be reached
    let new_balance = node1.online_reps.lock().unwrap().quorum_delta() - Amount::raw(1);
    let send1 = BlockBuilder::state()
        .account(*DEV_GENESIS_ACCOUNT)
        .previous(*DEV_GENESIS_HASH)
        .representative(*DEV_GENESIS_ACCOUNT)
        .balance(new_balance)
        .link(*DEV_GENESIS_ACCOUNT)
        .sign(&*DEV_GENESIS_KEY)
        .work(node1.work_generate_dev(*DEV_GENESIS_HASH))
        .build();

    assert_eq!(
        BlockStatus::Progress,
        node1.process_local(send1.clone()).unwrap()
    );

    node1
        .wallets
        .send_action2(
            &wallet_id,
            *DEV_GENESIS_ACCOUNT,
            *DEV_GENESIS_ACCOUNT,
            new_balance,
            0,
            true,
            None,
        )
        .unwrap();

    assert_timely_msg(
        Duration::from_secs(2),
        || node1.active.election(&send1.qualified_root()).is_some(),
        "Election not found",
    );

    let election = node1.active.election(&send1.qualified_root()).unwrap();
    assert!(!node1.active.confirmed(&election));
    assert_eq!(1, election.mutex.lock().unwrap().last_votes.len());
    assert_eq!(Amount::zero(), node1.balance(&DEV_GENESIS_ACCOUNT));
}

#[test]
fn bootstrap_connection_scaling() {
    let mut system = System::new();
    let node = system.make_node();

    assert_eq!(
        34,
        node.bootstrap_initiator
            .connections
            .target_connections(5000, 1)
    );
    assert_eq!(
        4,
        node.bootstrap_initiator
            .connections
            .target_connections(0, 1)
    );
    assert_eq!(
        64,
        node.bootstrap_initiator
            .connections
            .target_connections(50000, 1)
    );
    assert_eq!(
        64,
        node.bootstrap_initiator
            .connections
            .target_connections(10000000000, 1)
    );
    assert_eq!(
        32,
        node.bootstrap_initiator
            .connections
            .target_connections(5000, 0)
    );
    assert_eq!(
        1,
        node.bootstrap_initiator
            .connections
            .target_connections(0, 0)
    );
    assert_eq!(
        64,
        node.bootstrap_initiator
            .connections
            .target_connections(50000, 0)
    );
    assert_eq!(
        64,
        node.bootstrap_initiator
            .connections
            .target_connections(10000000000, 0)
    );
    assert_eq!(
        36,
        node.bootstrap_initiator
            .connections
            .target_connections(5000, 2)
    );
    assert_eq!(
        8,
        node.bootstrap_initiator
            .connections
            .target_connections(0, 2)
    );
    assert_eq!(
        64,
        node.bootstrap_initiator
            .connections
            .target_connections(50000, 2)
    );
    assert_eq!(
        64,
        node.bootstrap_initiator
            .connections
            .target_connections(10000000000, 2)
    );
}

#[test]
fn send_callback() {
    let mut system = System::new();
    let node = system.make_node();
    let wallet_id = node.wallets.wallet_ids()[0];
    let key2 = PrivateKey::new();

    node.wallets
        .insert_adhoc2(&wallet_id, &DEV_GENESIS_KEY.private_key(), true)
        .unwrap();
    node.wallets
        .insert_adhoc2(&wallet_id, &key2.private_key(), true)
        .unwrap();

    let send_result = node.wallets.send_action2(
        &wallet_id,
        *DEV_GENESIS_ACCOUNT,
        key2.account(),
        node.config.receive_minimum,
        0,
        true,
        None,
    );

    assert!(send_result.is_ok());

    assert_timely_msg(
        Duration::from_secs(10),
        || node.balance(&key2.account()).is_zero(),
        "balance is not zero",
    );

    assert_eq!(
        Amount::MAX - node.config.receive_minimum,
        node.balance(&DEV_GENESIS_ACCOUNT)
    );
}

// Test that nodes can disable representative voting
#[test]
fn no_voting() {
    let mut system = System::new();
    let node0 = system.make_node();
    let mut config = System::default_config();
    config.enable_voting = false;
    let node1 = system.build_node().config(config).finish();

    let wallet_id1 = node1.wallets.wallet_ids()[0];

    // Node1 has a rep
    node1
        .wallets
        .insert_adhoc2(&wallet_id1, &DEV_GENESIS_KEY.private_key(), true)
        .unwrap();
    let key1 = PrivateKey::new();
    node1
        .wallets
        .insert_adhoc2(&wallet_id1, &key1.private_key(), true)
        .unwrap();
    // Broadcast a confirm so others should know this is a rep node
    node1
        .wallets
        .send_action2(
            &wallet_id1,
            *DEV_GENESIS_ACCOUNT,
            key1.account(),
            Amount::nano(1),
            0,
            true,
            None,
        )
        .unwrap();

    assert_timely_eq(Duration::from_secs(10), || node0.active.len(), 0);
    assert_eq!(
        node0
            .stats
            .count(StatType::Message, DetailType::ConfirmAck, Direction::In),
        0
    );
}

#[test]
fn bootstrap_confirm_frontiers() {
    // create 2 separate systems, the 2 systems do not interact with each other automatically
    let mut system0 = System::new();
    let mut system1 = System::new();
    let node0 = system0.make_node();
    let node1 = system1.make_node();
    let wallet_id0 = node0.wallets.wallet_ids()[0];

    node0
        .wallets
        .insert_adhoc2(&wallet_id0, &DEV_GENESIS_KEY.private_key(), true)
        .unwrap();
    let key0 = PrivateKey::new();

    // create block to send 500 raw from genesis to key0 and save into node0 ledger without immediately triggering an election
    let send0 = Block::LegacySend(SendBlock::new(
        &DEV_GENESIS_HASH,
        &key0.account(),
        &(Amount::MAX - Amount::raw(500)),
        &DEV_GENESIS_KEY,
        node0.work_generate_dev(*DEV_GENESIS_HASH),
    ));

    assert_eq!(
        BlockStatus::Progress,
        node0.process_local(send0.clone()).unwrap()
    );

    // each system only has one node, so there should be no bootstrapping going on
    assert!(!node0.bootstrap_initiator.in_progress());
    assert!(!node1.bootstrap_initiator.in_progress());
    assert!(node1.active.len() == 0);

    // create a bootstrap connection from node1 to node0
    // this also has the side effect of adding node0 to node1's list of peers, which will trigger realtime connections too
    establish_tcp(&node1, &node0);
    node1
        .bootstrap_initiator
        .bootstrap2(node0.tcp_listener.local_address(), "".into());

    // Wait until the block is confirmed on node1
    let start_time = std::time::Instant::now();
    let timeout = Duration::from_secs(10);
    loop {
        {
            let tx = node1.store.tx_begin_read();
            if node1.ledger.confirmed().block_exists(&tx, &send0.hash()) {
                break;
            }
        }
        assert!(
            start_time.elapsed() < timeout,
            "Timeout waiting for block confirmation"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn bootstrap_fork_open() {
    let mut system = System::new();
    let mut node_config = System::default_config();
    let node0 = system.build_node().config(node_config.clone()).finish();
    let wallet_id0 = node0.wallets.wallet_ids()[0];
    node_config.peering_port = Some(get_available_port());
    let node1 = system.build_node().config(node_config).finish();
    let key0 = PrivateKey::new();

    let send0 = Block::LegacySend(SendBlock::new(
        &DEV_GENESIS_HASH,
        &key0.account(),
        &(Amount::MAX - Amount::raw(500)),
        &DEV_GENESIS_KEY,
        system
            .work
            .generate_dev2((*DEV_GENESIS_HASH).into())
            .unwrap(),
    ));

    let open0 = Block::LegacyOpen(OpenBlock::new(
        send0.hash(),
        PublicKey::from_bytes([1; 32]),
        key0.account(),
        &key0,
        system.work.generate_dev2(key0.public_key().into()).unwrap(),
    ));

    let open1 = Block::LegacyOpen(OpenBlock::new(
        send0.hash(),
        PublicKey::from_bytes([2; 32]),
        key0.account(),
        &key0,
        system.work.generate_dev2(key0.public_key().into()).unwrap(),
    ));

    // Both know about send0
    assert_eq!(
        BlockStatus::Progress,
        node0.process_local(send0.clone()).unwrap()
    );
    assert_eq!(
        BlockStatus::Progress,
        node1.process_local(send0.clone()).unwrap()
    );

    // Confirm send0 to allow starting and voting on the following blocks
    for node in [&node0, &node1] {
        start_election(&node, &node.latest(&*DEV_GENESIS_ACCOUNT));
        assert_timely_msg(
            Duration::from_secs(1),
            || node.active.election(&send0.qualified_root()).is_some(),
            "Election for send0 not found",
        );
        let election = node.active.election(&send0.qualified_root()).unwrap();
        node.active.force_confirm(&election);
        assert_timely_msg(
            Duration::from_secs(2),
            || node.active.len() == 0,
            "Active elections not empty",
        );
    }

    assert_timely_msg(
        Duration::from_secs(3),
        || node0.block_confirmed(&send0.hash()),
        "send0 not confirmed on node0",
    );

    // They disagree about open0/open1
    assert_eq!(
        BlockStatus::Progress,
        node0.process_local(open0.clone()).unwrap()
    );
    assert_eq!(
        BlockStatus::Progress,
        node1.process_local(open1.clone()).unwrap()
    );

    node0
        .wallets
        .insert_adhoc2(&wallet_id0, &DEV_GENESIS_KEY.private_key(), true)
        .unwrap();
    assert!(!node1
        .ledger
        .any()
        .block_exists_or_pruned(&node1.ledger.read_txn(), &open0.hash()));
    assert!(!node1.bootstrap_initiator.in_progress());

    node1
        .bootstrap_initiator
        .bootstrap2(node0.tcp_listener.local_address(), "".into());

    assert_timely_msg(
        Duration::from_secs(1),
        || node1.active.len() == 0,
        "Active elections not empty on node1",
    );

    assert_timely_msg(
        Duration::from_secs(10),
        || !node1.block_exists(&open1.hash()) && node1.block_exists(&open0.hash()),
        "Incorrect blocks exist on node1",
    );
}

#[test]
fn rep_self_vote() {
    let mut system = System::new();
    let mut node_config = System::default_config_without_backlog_population();
    node_config.online_weight_minimum = Amount::MAX;
    let node0 = system.build_node().config(node_config).finish();
    let wallet_id = node0.wallets.wallet_ids()[0];
    let rep_big = PrivateKey::new();

    let fund_big = Block::LegacySend(SendBlock::new(
        &DEV_GENESIS_HASH,
        &rep_big.account(),
        &Amount::raw(0xb000_0000_0000_0000_0000_0000_0000_0000),
        &DEV_GENESIS_KEY,
        system
            .work
            .generate_dev2((*DEV_GENESIS_HASH).into())
            .unwrap(),
    ));

    let open_big = Block::LegacyOpen(OpenBlock::new(
        fund_big.hash(),
        rep_big.public_key(),
        rep_big.account(),
        &rep_big,
        system
            .work
            .generate_dev2(rep_big.public_key().into())
            .unwrap(),
    ));

    assert_eq!(
        BlockStatus::Progress,
        node0.process_local(fund_big.clone()).unwrap()
    );
    assert_eq!(
        BlockStatus::Progress,
        node0.process_local(open_big.clone()).unwrap()
    );

    // Confirm both blocks, allowing voting on the upcoming block
    start_election(&node0, &open_big.hash());
    assert_timely_msg(
        Duration::from_secs(5),
        || node0.active.election(&open_big.qualified_root()).is_some(),
        "Election for open_big not found",
    );
    let election = node0.active.election(&open_big.qualified_root()).unwrap();
    node0.active.force_confirm(&election);

    node0
        .wallets
        .insert_adhoc2(&wallet_id, &rep_big.private_key(), true)
        .unwrap();
    node0
        .wallets
        .insert_adhoc2(&wallet_id, &DEV_GENESIS_KEY.private_key(), true)
        .unwrap();
    assert_eq!(node0.wallets.voting_reps_count(), 2);

    let block0 = Block::LegacySend(SendBlock::new(
        &fund_big.hash(),
        &rep_big.account(),
        &Amount::raw(0x6000_0000_0000_0000_0000_0000_0000_0000),
        &DEV_GENESIS_KEY,
        system.work.generate_dev2(fund_big.hash().into()).unwrap(),
    ));

    assert_eq!(
        BlockStatus::Progress,
        node0.process_local(block0.clone()).unwrap()
    );

    let election1 = start_election(&node0, &block0.hash());

    // Wait until representatives are activated & make vote
    assert_timely_eq(Duration::from_secs(1), || election1.vote_count(), 3);

    let rep_votes = election1.mutex.lock().unwrap().last_votes.clone();
    assert!(rep_votes.contains_key(&DEV_GENESIS_KEY.public_key()));
    assert!(rep_votes.contains_key(&rep_big.public_key()));
}

#[test]
fn fork_bootstrap_flip() {
    let mut system = System::new();
    let config0 = System::default_config_without_backlog_population();
    let mut node_flags = NodeFlags::default();
    node_flags.disable_bootstrap_bulk_push_client = true;
    node_flags.disable_lazy_bootstrap = true;

    let node1 = system
        .build_node()
        .config(config0)
        .flags(node_flags.clone())
        .finish();
    let wallet_id1 = node1.wallets.wallet_ids()[0];
    node1
        .wallets
        .insert_adhoc2(&wallet_id1, &DEV_GENESIS_KEY.private_key(), true)
        .unwrap();

    let mut config1 = System::default_config();
    config1.peering_port = Some(get_available_port());
    let node2 = system.make_disconnected_node();
    //let node2 = system.build_disconnected_node(config1, node_flags);
    node1
        .wallets
        .insert_adhoc2(&wallet_id1, &DEV_GENESIS_KEY.private_key(), true)
        .unwrap();

    let latest = node1.latest(&DEV_GENESIS_ACCOUNT);
    let key1 = PrivateKey::new();
    let send1 = Block::LegacySend(SendBlock::new(
        &latest,
        &key1.account(),
        &(Amount::MAX - Amount::raw(1_000_000)),
        &DEV_GENESIS_KEY,
        system.work.generate_dev2(latest.into()).unwrap(),
    ));

    let key2 = PrivateKey::new();
    let send2 = Block::LegacySend(SendBlock::new(
        &latest,
        &key2.account(),
        &(Amount::MAX - Amount::raw(1_000_000)),
        &DEV_GENESIS_KEY,
        system.work.generate_dev2(latest.into()).unwrap(),
    ));

    // Insert but don't rebroadcast, simulating settled blocks
    assert_eq!(
        BlockStatus::Progress,
        node1.process_local(send1.clone()).unwrap()
    );
    assert_eq!(
        BlockStatus::Progress,
        node2.process_local(send2.clone()).unwrap()
    );

    node1.confirm(send1.hash());
    assert_timely_msg(
        Duration::from_secs(1),
        || node1.block_exists(&send1.hash()),
        "send1 not found on node1",
    );
    assert_timely_msg(
        Duration::from_secs(1),
        || node2.block_exists(&send2.hash()),
        "send2 not found on node2",
    );

    // Additionally add new peer to confirm & replace bootstrap block
    //node2.network.merge_peer(node1.network.endpoint());
    establish_tcp(&node2, &node1);

    assert_timely_msg(
        Duration::from_secs(10),
        || node2.block_exists(&send1.hash()),
        "send1 not found on node2 after bootstrap",
    );
}

#[test]
fn fork_multi_flip() {
    let mut system = System::new();
    let mut config = System::default_config_without_backlog_population();
    let flags = NodeFlags::default();
    let node1 = system
        .build_node()
        .config(config.clone())
        .flags(flags.clone())
        .finish();
    let wallet_id1 = node1.wallets.wallet_ids()[0];
    config.peering_port = Some(get_available_port());
    let node2 = system.build_node().config(config).flags(flags).finish();

    let key1 = PrivateKey::new();
    let send1 = Block::LegacySend(SendBlock::new(
        &DEV_GENESIS_HASH,
        &key1.account(),
        &(Amount::MAX - Amount::raw(100)),
        &DEV_GENESIS_KEY,
        system
            .work
            .generate_dev2((*DEV_GENESIS_HASH).into())
            .unwrap(),
    ));

    let key2 = PrivateKey::new();
    let send2 = Block::LegacySend(SendBlock::new(
        &DEV_GENESIS_HASH,
        &key2.account(),
        &(Amount::MAX - Amount::raw(100)),
        &DEV_GENESIS_KEY,
        system
            .work
            .generate_dev2((*DEV_GENESIS_HASH).into())
            .unwrap(),
    ));

    let send3 = Block::LegacySend(SendBlock::new(
        &send2.hash(),
        &key2.account(),
        &(Amount::MAX - Amount::raw(100)),
        &DEV_GENESIS_KEY,
        system.work.generate_dev2(send2.hash().into()).unwrap(),
    ));

    assert_eq!(
        BlockStatus::Progress,
        node1.process_local(send1.clone()).unwrap()
    );
    // Node2 has two blocks that will be rolled back by node1's vote
    assert_eq!(
        BlockStatus::Progress,
        node2.process_local(send2.clone()).unwrap()
    );
    assert_eq!(
        BlockStatus::Progress,
        node2.process_local(send3.clone()).unwrap()
    );

    node1
        .wallets
        .insert_adhoc2(&wallet_id1, &DEV_GENESIS_KEY.private_key(), true)
        .unwrap(); // Insert voting key into node1

    let election = start_election(&node2, &send2.hash());

    assert_timely_msg(
        Duration::from_secs(5),
        || {
            election
                .mutex
                .lock()
                .unwrap()
                .last_blocks
                .contains_key(&send1.hash())
        },
        "election doesn't contain send1",
    );

    node1.confirm(send1.hash());
    assert_timely_msg(
        Duration::from_secs(10),
        || {
            node2
                .ledger
                .any()
                .block_exists_or_pruned(&node2.ledger.read_txn(), &send1.hash())
        },
        "send1 not found on node2",
    );
    assert!(!node2
        .ledger
        .any()
        .block_exists(&node2.ledger.read_txn(), &send2.hash()));
    assert!(!node2
        .ledger
        .any()
        .block_exists_or_pruned(&node2.ledger.read_txn(), &send3.hash()));

    let winner = election.winner_hash().unwrap();
    assert_eq!(send1.hash(), winner);
    assert_eq!(
        Amount::MAX - Amount::raw(100),
        *election
            .mutex
            .lock()
            .unwrap()
            .last_tally
            .get(&send1.hash())
            .unwrap()
    );
}

// In test case there used to be a race condition, it was worked around in:.
// https://github.com/nanocurrency/nano-node/pull/4091
// The election and the processing of block send2 happen in parallel.
// Usually the election happens first and the send2 block is added to the election.
// However, if the send2 block is processed before the election is started then
// there is a race somewhere and the election might not notice the send2 block.
// The test case can be made to pass by ensuring the election is started before the send2 is processed.
// However, is this a problem with the test case or this is a problem with the node handling of forks?
#[test]
fn fork_publish_inactive() {
    let mut system = System::new();
    let node = system.make_node();
    let key1 = PrivateKey::new();
    let key2 = PrivateKey::new();

    let send1 = Block::LegacySend(SendBlock::new(
        &DEV_GENESIS_HASH,
        &key1.account(),
        &(Amount::MAX - Amount::raw(100)),
        &DEV_GENESIS_KEY,
        system
            .work
            .generate_dev2((*DEV_GENESIS_HASH).into())
            .unwrap(),
    ));

    let send2 = Block::LegacySend(SendBlock::new(
        &DEV_GENESIS_HASH,
        &key2.account(),
        &(Amount::MAX - Amount::raw(100)),
        &DEV_GENESIS_KEY,
        system
            .work
            .generate_dev2((*DEV_GENESIS_HASH).into())
            .unwrap(),
    ));

    node.process_active(send1.clone());
    assert_timely_msg(
        Duration::from_secs(5),
        || node.block_exists(&send1.hash()),
        "send1 not found",
    );

    assert_timely_msg(
        Duration::from_secs(5),
        || node.active.election(&send1.qualified_root()).is_some(),
        "election not found",
    );

    let election = node.active.election(&send1.qualified_root()).unwrap();

    assert_eq!(
        node.process_local(send2.clone()).unwrap(),
        BlockStatus::Fork
    );

    assert_timely_eq(
        Duration::from_secs(5),
        || election.mutex.lock().unwrap().last_blocks.len(),
        2,
    );

    let find_block = |hash: BlockHash| {
        election
            .mutex
            .lock()
            .unwrap()
            .last_blocks
            .contains_key(&hash)
    };

    assert!(find_block(send1.hash()));
    assert!(find_block(send2.hash()));

    assert_eq!(election.winner_hash().unwrap(), send1.hash());
    assert_ne!(election.winner_hash().unwrap(), send2.hash());
}

#[test]
fn unlock_search() {
    let mut system = System::new();
    let node = system.make_node();
    let wallet_id = node.wallets.wallet_ids()[0];
    let key2 = PrivateKey::new();
    let balance = node.balance(&DEV_GENESIS_ACCOUNT);

    node.wallets.rekey(&wallet_id, "").unwrap();
    node.wallets
        .insert_adhoc2(&wallet_id, &DEV_GENESIS_KEY.private_key(), true)
        .unwrap();

    node.wallets
        .send_action2(
            &wallet_id,
            *DEV_GENESIS_ACCOUNT,
            key2.account(),
            node.config.receive_minimum,
            0,
            true,
            None,
        )
        .unwrap();

    assert_timely_msg(
        Duration::from_secs(10),
        || node.balance(&DEV_GENESIS_ACCOUNT) != balance,
        "balance not updated",
    );

    assert_timely_eq(Duration::from_secs(10), || node.active.len(), 0);

    node.wallets
        .insert_adhoc2(&wallet_id, &key2.private_key(), true)
        .unwrap();
    //node.wallets
    //    .set_password(&wallet_id, &KeyPair::new().private_key())
    //    .unwrap();
    node.wallets.enter_password(wallet_id, "").unwrap();

    assert_timely_msg(
        Duration::from_secs(10),
        || !node.balance(&key2.account()).is_zero(),
        "balance is still zero",
    );
}

#[test]
fn search_receivable_confirmed() {
    let mut system = System::new();
    let config = System::default_config_without_backlog_population();
    let node = system.build_node().config(config).finish();
    let wallet_id = node.wallets.wallet_ids()[0];
    let key2 = PrivateKey::new();
    node.wallets
        .insert_adhoc2(&wallet_id, &DEV_GENESIS_KEY.private_key(), true)
        .unwrap();

    let send1 = node
        .wallets
        .send_action2(
            &wallet_id,
            *DEV_GENESIS_ACCOUNT,
            key2.account(),
            node.config.receive_minimum,
            0,
            true,
            None,
        )
        .unwrap();
    assert_timely(Duration::from_secs(5), || {
        node.block_hashes_confirmed(&[send1.hash()])
    });

    let send2 = node
        .wallets
        .send_action2(
            &wallet_id,
            *DEV_GENESIS_ACCOUNT,
            key2.account(),
            node.config.receive_minimum,
            0,
            true,
            None,
        )
        .unwrap();
    assert_timely(Duration::from_secs(5), || {
        node.block_hashes_confirmed(&[send2.hash()])
    });

    node.wallets
        .remove_key(&wallet_id, &*DEV_GENESIS_PUB_KEY)
        .unwrap();

    node.wallets
        .insert_adhoc2(&wallet_id, &key2.private_key(), true)
        .unwrap();

    node.wallets.search_receivable_wallet(wallet_id).unwrap();

    assert_timely(Duration::from_secs(5), || {
        !node.active.election(&send1.qualified_root()).is_some()
    });

    assert_timely(Duration::from_secs(5), || {
        !node.active.election(&send2.qualified_root()).is_some()
    });

    assert_timely_eq(
        Duration::from_secs(5),
        || node.balance(&key2.account()),
        node.config.receive_minimum * 2,
    );
}

#[test]
fn search_receivable_pruned() {
    let mut system = System::new();
    let config1 = System::default_config_without_backlog_population();
    let node1 = system.build_node().config(config1).finish();
    let wallet_id = node1.wallets.wallet_ids()[0];

    let mut config2 = System::default_config();
    config2.enable_voting = false; // Remove after allowing pruned voting
    let mut flags = NodeFlags::default();
    flags.enable_pruning = true;
    let node2 = system.build_node().config(config2).flags(flags).finish();
    let wallet_id2 = node2.wallets.wallet_ids()[0];

    let key2 = PrivateKey::new();
    node1
        .wallets
        .insert_adhoc2(&wallet_id, &DEV_GENESIS_KEY.private_key(), true)
        .unwrap();

    let send1 = node1
        .wallets
        .send_action2(
            &wallet_id,
            *DEV_GENESIS_ACCOUNT,
            key2.account(),
            node2.config.receive_minimum,
            0,
            true,
            None,
        )
        .unwrap();

    let send2 = node1
        .wallets
        .send_action2(
            &wallet_id,
            *DEV_GENESIS_ACCOUNT,
            key2.account(),
            node2.config.receive_minimum,
            0,
            true,
            None,
        )
        .unwrap();

    // Confirmation
    assert_timely(Duration::from_secs(10), || {
        node1.active.len() == 0 && node2.active.len() == 0
    });
    assert_timely(Duration::from_secs(5), || {
        node1
            .ledger
            .confirmed()
            .block_exists(&node1.ledger.read_txn(), &send2.hash())
    });
    assert_timely_eq(Duration::from_secs(5), || node2.ledger.cemented_count(), 3);

    node1
        .wallets
        .remove_key(&wallet_id, &*DEV_GENESIS_PUB_KEY)
        .unwrap();

    // Pruning
    {
        let mut transaction = node2.store.tx_begin_write();
        assert_eq!(
            1,
            node2
                .ledger
                .pruning_action(&mut transaction, &send1.hash(), 1)
        );
    }
    assert_eq!(1, node2.ledger.pruned_count());
    assert!(node2
        .ledger
        .any()
        .block_exists_or_pruned(&node2.ledger.read_txn(), &send1.hash())); // true for pruned

    // Receive pruned block
    node2
        .wallets
        .insert_adhoc2(&wallet_id2, &key2.private_key(), true)
        .unwrap();

    node2.wallets.search_receivable_wallet(wallet_id2).unwrap();
    assert_timely_eq(
        Duration::from_secs(10),
        || node2.balance(&key2.account()),
        node2.config.receive_minimum * 2,
    );
}

#[test]
fn search_receivable() {
    let mut system = System::new();
    let node = system.make_node();
    let wallet_id = node.wallets.wallet_ids()[0];
    let key2 = PrivateKey::new();
    node.wallets
        .insert_adhoc2(&wallet_id, &DEV_GENESIS_KEY.private_key(), true)
        .unwrap();
    node.wallets
        .send_action2(
            &wallet_id,
            *DEV_GENESIS_ACCOUNT,
            key2.account(),
            node.config.receive_minimum,
            0,
            true,
            None,
        )
        .unwrap();

    node.wallets
        .insert_adhoc2(&wallet_id, &key2.private_key(), true)
        .unwrap();

    node.wallets.search_receivable_wallet(wallet_id).unwrap();

    assert_timely_msg(
        Duration::from_secs(10),
        || !node.balance(&key2.account()).is_zero(),
        "balance is still zero",
    );
}

#[test]
fn search_receivable_same() {
    let mut system = System::new();
    let node = system.make_node();
    let wallet_id = node.wallets.wallet_ids()[0];
    let key2 = PrivateKey::new();
    node.wallets
        .insert_adhoc2(&wallet_id, &DEV_GENESIS_KEY.private_key(), true)
        .unwrap();
    let send_result1 = node.wallets.send_action2(
        &wallet_id,
        *DEV_GENESIS_ACCOUNT,
        key2.account(),
        node.config.receive_minimum,
        0,
        true,
        None,
    );
    assert!(send_result1.is_ok());
    let send_result2 = node.wallets.send_action2(
        &wallet_id,
        *DEV_GENESIS_ACCOUNT,
        key2.account(),
        node.config.receive_minimum,
        0,
        true,
        None,
    );
    assert!(send_result2.is_ok());
    node.wallets
        .insert_adhoc2(&wallet_id, &key2.private_key(), true)
        .unwrap();
    node.wallets.search_receivable_wallet(wallet_id).unwrap();

    assert_timely_msg(
        Duration::from_secs(10),
        || node.balance(&key2.account()) == node.config.receive_minimum * 2,
        "balance is not equal to twice the receive minimum",
    );
}

#[test]
fn search_receivable_multiple() {
    let mut system = System::new();
    let node = system.make_node();
    let wallet_id = node.wallets.wallet_ids()[0];
    let key2 = PrivateKey::new();
    let key3 = PrivateKey::new();
    node.wallets
        .insert_adhoc2(&wallet_id, &DEV_GENESIS_KEY.private_key(), true)
        .unwrap();
    node.wallets
        .insert_adhoc2(&wallet_id, &key3.private_key(), true)
        .unwrap();
    node.wallets
        .send_action2(
            &wallet_id,
            *DEV_GENESIS_ACCOUNT,
            key3.account(),
            node.config.receive_minimum,
            0,
            true,
            None,
        )
        .unwrap();
    assert_timely_msg(
        Duration::from_secs(10),
        || !node.balance(&key3.account()).is_zero(),
        "key3 balance is still zero",
    );
    node.wallets
        .send_action2(
            &wallet_id,
            *DEV_GENESIS_ACCOUNT,
            key2.account(),
            node.config.receive_minimum,
            0,
            true,
            None,
        )
        .unwrap();
    node.wallets
        .send_action2(
            &wallet_id,
            key3.account(),
            key2.account(),
            node.config.receive_minimum,
            0,
            true,
            None,
        )
        .unwrap();
    node.wallets
        .insert_adhoc2(&wallet_id, &key2.private_key(), true)
        .unwrap();
    node.wallets.search_receivable_wallet(wallet_id).unwrap();

    assert_timely_msg(
        Duration::from_secs(10),
        || node.balance(&key2.account()) == node.config.receive_minimum * 2,
        "key2 balance is not equal to twice the receive minimum",
    );
}

#[test]
fn auto_bootstrap_age() {
    let mut system = System::new();
    let config = System::default_config_without_backlog_population();
    let mut node_flags = NodeFlags::default();
    node_flags.disable_bootstrap_bulk_push_client = true;
    node_flags.disable_lazy_bootstrap = true;
    node_flags.bootstrap_interval = 1;

    let node0 = system
        .build_node()
        .config(config.clone())
        .flags(node_flags.clone())
        .finish();
    let _node1 = system.make_node();

    // Wait for at least 3 bootstrap attempts with frontiers age
    assert_timely_msg(
        Duration::from_secs(10),
        || {
            node0.stats.count(
                StatType::Bootstrap,
                DetailType::InitiateLegacyAge,
                Direction::Out,
            ) >= 3
        },
        "not enough bootstrap attempts with frontiers age",
    );

    // Check that there are more attempts with frontiers age than without
    assert!(
        node0.stats.count(
            StatType::Bootstrap,
            DetailType::InitiateLegacyAge,
            Direction::Out
        ) >= node0
            .stats
            .count(StatType::Bootstrap, DetailType::Initiate, Direction::Out),
        "unexpected ratio of bootstrap attempts"
    );
}

#[test]
fn auto_bootstrap_reverse() {
    let mut system = System::new();
    let config = System::default_config_without_backlog_population();
    let mut node_flags = NodeFlags::default();
    node_flags.disable_bootstrap_bulk_push_client = true;
    node_flags.disable_lazy_bootstrap = true;

    let node0 = system
        .build_node()
        .config(config.clone())
        .flags(node_flags.clone())
        .finish();
    let wallet_id = node0.wallets.wallet_ids()[0];
    let key2 = PrivateKey::new();

    node0
        .wallets
        .insert_adhoc2(&wallet_id, &DEV_GENESIS_KEY.private_key(), true)
        .unwrap();
    node0
        .wallets
        .insert_adhoc2(&wallet_id, &key2.private_key(), true)
        .unwrap();

    let node1 = system.make_node();

    let send_result = node0.wallets.send_action2(
        &wallet_id,
        *DEV_GENESIS_ACCOUNT,
        key2.account(),
        node0.config.receive_minimum,
        0,
        true,
        None,
    );
    assert!(send_result.is_ok());

    establish_tcp(&node0, &node1);

    assert_timely_msg(
        Duration::from_secs(10),
        || node1.balance(&key2.account()) == node0.config.receive_minimum,
        "balance not synced",
    );
}

#[test]
fn quick_confirm() {
    let mut system = System::new();
    let node1 = system.make_node();
    let wallet_id = node1.wallets.wallet_ids()[0];
    let key = PrivateKey::new();
    let previous = node1.latest(&DEV_GENESIS_ACCOUNT);
    let genesis_start_balance = node1.balance(&DEV_GENESIS_ACCOUNT);

    node1
        .wallets
        .insert_adhoc2(&wallet_id, &key.private_key(), true)
        .unwrap();
    node1
        .wallets
        .insert_adhoc2(&wallet_id, &DEV_GENESIS_KEY.private_key(), true)
        .unwrap();

    let send = Block::LegacySend(SendBlock::new(
        &previous,
        &key.account(),
        &(node1.online_reps.lock().unwrap().quorum_delta() + Amount::raw(1)),
        &DEV_GENESIS_KEY,
        system.work.generate_dev2(previous.into()).unwrap(),
    ));

    node1.process_active(send.clone());

    assert_timely_msg(
        Duration::from_secs(10),
        || !node1.balance(&key.account()).is_zero(),
        "balance is still zero",
    );

    assert_eq!(
        node1.balance(&DEV_GENESIS_ACCOUNT),
        node1.online_reps.lock().unwrap().quorum_delta() + Amount::raw(1)
    );

    assert_eq!(
        node1.balance(&key.account()),
        genesis_start_balance - (node1.online_reps.lock().unwrap().quorum_delta() + Amount::raw(1))
    );
}

#[test]
fn send_out_of_order() {
    let mut system = System::new();
    let node1 = system.make_node();
    let key2 = PrivateKey::new();

    let send1 = Block::LegacySend(SendBlock::new(
        &DEV_GENESIS_HASH,
        &key2.account(),
        &(Amount::MAX - node1.config.receive_minimum),
        &DEV_GENESIS_KEY,
        system
            .work
            .generate_dev2((*DEV_GENESIS_HASH).into())
            .unwrap(),
    ));

    let send2 = Block::LegacySend(SendBlock::new(
        &send1.hash(),
        &key2.account(),
        &(Amount::MAX - node1.config.receive_minimum * 2),
        &DEV_GENESIS_KEY,
        system.work.generate_dev2(send1.hash().into()).unwrap(),
    ));

    let send3 = Block::LegacySend(SendBlock::new(
        &send2.hash(),
        &key2.account(),
        &(Amount::MAX - node1.config.receive_minimum * 3),
        &DEV_GENESIS_KEY,
        system.work.generate_dev2(send2.hash().into()).unwrap(),
    ));

    node1.process_active(send3.clone());
    node1.process_active(send2.clone());
    node1.process_active(send1.clone());

    assert_timely_msg(
        Duration::from_secs(10),
        || {
            system.nodes.iter().all(|node| {
                node.balance(&DEV_GENESIS_ACCOUNT) == Amount::MAX - node1.config.receive_minimum * 3
            })
        },
        "balance is incorrect on at least one node",
    );
}

#[test]
fn send_single_observing_peer() {
    let mut system = System::new();
    let key2 = PrivateKey::new();
    let node1 = system.make_node();
    let node2 = system.make_node();
    let wallet_id1 = node1.wallets.wallet_ids()[0];
    let wallet_id2 = node2.wallets.wallet_ids()[0];

    node1
        .wallets
        .insert_adhoc2(&wallet_id1, &DEV_GENESIS_KEY.private_key(), true)
        .unwrap();
    node2
        .wallets
        .insert_adhoc2(&wallet_id2, &key2.private_key(), true)
        .unwrap();

    node1
        .wallets
        .send_action2(
            &wallet_id1,
            *DEV_GENESIS_ACCOUNT,
            key2.account(),
            node1.config.receive_minimum,
            0,
            true,
            None,
        )
        .unwrap();

    assert_eq!(
        Amount::MAX - node1.config.receive_minimum,
        node1.balance(&DEV_GENESIS_ACCOUNT)
    );

    assert!(node1.balance(&key2.account()).is_zero());

    assert_timely_msg(
        Duration::from_secs(10),
        || {
            system
                .nodes
                .iter()
                .all(|node| !node.balance(&key2.account()).is_zero())
        },
        "balance is still zero on at least one node",
    );
}

#[test]
fn send_single() {
    let mut system = System::new();
    let key2 = PrivateKey::new();
    let node1 = system.make_node();
    let node2 = system.make_node();
    let wallet_id1 = node1.wallets.wallet_ids()[0];
    let wallet_id2 = node2.wallets.wallet_ids()[0];

    node1
        .wallets
        .insert_adhoc2(&wallet_id1, &DEV_GENESIS_KEY.private_key(), true)
        .unwrap();
    node2
        .wallets
        .insert_adhoc2(&wallet_id2, &key2.private_key(), true)
        .unwrap();

    node1
        .wallets
        .send_action2(
            &wallet_id1,
            *DEV_GENESIS_ACCOUNT,
            key2.account(),
            node1.config.receive_minimum,
            0,
            true,
            None,
        )
        .unwrap();

    assert_eq!(
        Amount::MAX - node1.config.receive_minimum,
        node1.balance(&DEV_GENESIS_ACCOUNT)
    );

    assert!(node1.balance(&key2.account()).is_zero());

    assert_timely_msg(
        Duration::from_secs(10),
        || !node1.balance(&key2.account()).is_zero(),
        "balance is still zero",
    );
}

#[test]
fn send_self() {
    let mut system = System::new();
    let key2 = PrivateKey::new();
    let node = system.make_node();
    let wallet_id = node.wallets.wallet_ids()[0];
    node.wallets
        .insert_adhoc2(&wallet_id, &DEV_GENESIS_KEY.private_key(), true)
        .unwrap();
    node.wallets
        .insert_adhoc2(&wallet_id, &key2.private_key(), true)
        .unwrap();

    node.wallets
        .send_action2(
            &wallet_id,
            *DEV_GENESIS_ACCOUNT,
            key2.account(),
            node.config.receive_minimum,
            0,
            true,
            None,
        )
        .unwrap();

    assert_timely_msg(
        Duration::from_secs(10),
        || !node.balance(&key2.account()).is_zero(),
        "balance is still zero",
    );

    assert_eq!(
        Amount::MAX - node.config.receive_minimum,
        node.balance(&DEV_GENESIS_ACCOUNT)
    );
}

#[test]
fn balance() {
    let mut system = System::new();
    let node = system.make_node();

    let wallet_id = node.wallets.wallet_ids()[0];
    node.wallets
        .insert_adhoc2(&wallet_id, &DEV_GENESIS_KEY.private_key(), true)
        .unwrap();

    let balance = node.balance(&DEV_GENESIS_KEY.account());

    assert_eq!(Amount::MAX, balance);
}

#[test]
fn work_generate() {
    let mut system = System::new();
    let node = system.make_node();
    let root = Root::from(1);

    // Test with higher difficulty
    {
        let difficulty =
            DifficultyV1::from_multiplier(1.5, node.network_params.work.threshold_base());
        let work = node.distributed_work.make_blocking(root, difficulty, None);
        assert!(work.is_some());
        let work = work.unwrap();
        assert!(node.network_params.work.difficulty(&root, work) >= difficulty);
    }

    // Test with lower difficulty
    {
        let difficulty =
            DifficultyV1::from_multiplier(0.5, node.network_params.work.threshold_base());
        let mut work;
        loop {
            work = node.distributed_work.make_blocking(root, difficulty, None);
            if let Some(work_value) = work {
                if node.network_params.work.difficulty(&root, work_value)
                    < node.network_params.work.threshold_base()
                {
                    break;
                }
            }
        }
        let work = work.unwrap();
        assert!(node.network_params.work.difficulty(&root, work) >= difficulty);
        assert!(
            node.network_params.work.difficulty(&root, work)
                < node.network_params.work.threshold_base()
        );
    }
}

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

    let key1 = PrivateKey::new();
    let latest_hash = *DEV_GENESIS_HASH;

    let send1 = Block::LegacySend(SendBlock::new(
        &latest_hash,
        &key1.account(),
        &(Amount::MAX - Amount::nano(1000)),
        &DEV_GENESIS_KEY,
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
    assert_timely_msg(
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
        node2.block_exists(&send_hash)
    });

    // Connect the nodes and check that the block is propagated
    node1
        .peer_connector
        .connect_to(node2.tcp_listener.local_address());
    assert_timely_msg(
        Duration::from_secs(5),
        || {
            node1
                .network_info
                .read()
                .unwrap()
                .find_node_id(&node2.get_node_id())
                .is_some()
        },
        "node2 not connected",
    );
    assert_timely_msg(
        Duration::from_secs(10),
        || node2.block_exists(&send_hash),
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
            key4.into(),
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
            key1.into(),
            node1.config.receive_minimum,
            0,
            true,
            None,
        )
        .unwrap();
    assert_timely_msg(
        Duration::from_secs(30),
        || {
            node3.balance(&key1.into()) == node1.config.receive_minimum
                && node2.balance(&key1.into()) == node1.config.receive_minimum
                && node1.balance(&key1.into()) == node1.config.receive_minimum
        },
        "balances are wrong",
    );
    assert_eq!(node1.config.receive_minimum, node1.ledger.weight(&key1));
    assert_eq!(node1.config.receive_minimum, node2.ledger.weight(&key1));
    assert_eq!(node1.config.receive_minimum, node3.ledger.weight(&key1));

    let send1 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        block.hash(),
        *DEV_GENESIS_PUB_KEY,
        (Amount::MAX / 4) - (node1.config.receive_minimum * 2),
        Account::from(key1).into(),
        &DEV_GENESIS_KEY,
        node1.work_generate_dev(block.hash()),
    ));

    node1.process(send1.clone()).unwrap();
    node2.process(send1.clone()).unwrap();
    node3.process(send1.clone()).unwrap();

    let key2 = node3
        .wallets
        .deterministic_insert2(&wallet_id3, true)
        .unwrap();

    let send2 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        block.hash(),
        *DEV_GENESIS_PUB_KEY,
        (Amount::MAX / 4) - (node1.config.receive_minimum * 2),
        Account::from(key2).into(),
        &DEV_GENESIS_KEY,
        node1.work_generate_dev(block.hash()),
    ));
    let vote = Vote::new(&PrivateKey::new(), 0, 0, vec![send2.hash()]);
    let confirm = Message::ConfirmAck(ConfirmAck::new_with_own_vote(vote));
    let channel = node2
        .network_info
        .read()
        .unwrap()
        .find_node_id(&node3.node_id())
        .unwrap()
        .clone();
    node2.message_publisher.lock().unwrap().try_send(
        channel.channel_id(),
        &confirm,
        DropPolicy::ShouldNotDrop,
        TrafficType::Generic,
    );

    assert_timely_msg(
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

#[test]
fn fork_open() {
    let mut system = System::new();
    let node = system.make_node();
    let wallet_id = node.wallets.wallet_ids()[0];

    // create block send1, to send all the balance from genesis to key1
    // this is done to ensure that the open block(s) cannot be voted on and confirmed
    let key1 = PrivateKey::new();
    let send1 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::zero(),
        key1.account().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(*DEV_GENESIS_HASH),
    ));

    let channel = make_fake_channel(&node);

    node.inbound_message_queue.put(
        Message::Publish(Publish::new_forward(send1.clone())),
        channel.info.clone(),
    );

    assert_timely_msg(
        Duration::from_secs(5),
        || node.active.election(&send1.qualified_root()).is_some(),
        "election not found",
    );
    let election = node.active.election(&send1.qualified_root()).unwrap();
    node.active.force_confirm(&election);
    assert_timely_eq(Duration::from_secs(5), || node.active.len(), 0);

    // register key for genesis account, not sure why we do this, it seems needless,
    // since the genesis account at this stage has zero voting weight
    node.wallets
        .insert_adhoc2(&wallet_id, &DEV_GENESIS_KEY.private_key(), true)
        .unwrap();

    // create the 1st open block to receive send1, which should be regarded as the winner just because it is first
    let open1 = Block::State(StateBlock::new(
        key1.account(),
        BlockHash::zero(),
        1.into(),
        Amount::MAX,
        send1.hash().into(),
        &key1,
        node.work_generate_dev(&key1),
    ));
    node.inbound_message_queue.put(
        Message::Publish(Publish::new_forward(open1.clone())),
        channel.info.clone(),
    );
    assert_timely_eq(Duration::from_secs(5), || node.active.len(), 1);

    // create 2nd open block, which is a fork of open1 block
    // create the 1st open block to receive send1, which should be regarded as the winner just because it is first
    let open2 = Block::State(StateBlock::new(
        key1.account(),
        BlockHash::zero(),
        2.into(),
        Amount::MAX,
        send1.hash().into(),
        &key1,
        node.work_generate_dev(&key1),
    ));
    node.inbound_message_queue.put(
        Message::Publish(Publish::new_forward(open2.clone())),
        channel.info.clone(),
    );
    assert_timely_msg(
        Duration::from_secs(5),
        || node.active.election(&open2.qualified_root()).is_some(),
        "no election for open2",
    );

    let election = node.active.election(&open2.qualified_root()).unwrap();
    // we expect to find 2 blocks in the election and we expect the first block to be the winner just because it was first
    assert_timely_eq(
        Duration::from_secs(5),
        || election.mutex.lock().unwrap().last_blocks.len(),
        2,
    );
    assert_eq!(open1.hash(), election.winner_hash().unwrap());

    // wait for a second and check that the election did not get confirmed
    sleep(Duration::from_millis(1000));
    assert_eq!(node.active.confirmed(&election), false);

    // check that only the first block is saved to the ledger
    assert_timely_msg(
        Duration::from_secs(5),
        || node.block_exists(&open1.hash()),
        "open1 not in ledger",
    );
    assert_eq!(node.block_exists(&open2.hash()), false);
}

#[test]
fn online_reps_rep_crawler() {
    let mut system = System::new();
    let mut flags = NodeFlags::default();
    flags.disable_rep_crawler = true;
    let node = system.build_node().flags(flags).finish();
    let vote = Arc::new(Vote::new(
        &DEV_GENESIS_KEY,
        milliseconds_since_epoch(),
        0,
        vec![*DEV_GENESIS_HASH],
    ));
    assert_eq!(
        Amount::zero(),
        node.online_reps.lock().unwrap().online_weight()
    );

    // Without rep crawler
    let channel = make_fake_channel(&node);
    node.vote_processor
        .vote_blocking(&vote, channel.channel_id(), VoteSource::Live);
    assert_eq!(
        Amount::zero(),
        node.online_reps.lock().unwrap().online_weight()
    );

    // After inserting to rep crawler
    node.rep_crawler
        .force_query(*DEV_GENESIS_HASH, channel.channel_id());
    node.vote_processor
        .vote_blocking(&vote, channel.channel_id(), VoteSource::Live);
    assert_eq!(
        Amount::MAX,
        node.online_reps.lock().unwrap().online_weight()
    );
}

#[test]
fn online_reps_election() {
    let mut system = System::new();
    let mut flags = NodeFlags::default();
    flags.disable_rep_crawler = true;
    let node = system.build_node().flags(flags).finish();

    // Start election
    let key = PrivateKey::new();
    let send1 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::nano(1000),
        key.account().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(*DEV_GENESIS_HASH),
    ));

    node.process_active(send1.clone());
    assert_timely_eq(Duration::from_secs(5), || node.active.len(), 1);

    // Process vote for ongoing election
    let vote = Arc::new(Vote::new(
        &DEV_GENESIS_KEY,
        milliseconds_since_epoch(),
        0,
        vec![send1.hash()],
    ));
    assert_eq!(
        Amount::zero(),
        node.online_reps.lock().unwrap().online_weight()
    );

    let channel = make_fake_channel(&node);
    node.vote_processor
        .vote_blocking(&vote, channel.channel_id(), VoteSource::Live);

    assert_eq!(
        Amount::MAX - Amount::nano(1000),
        node.online_reps.lock().unwrap().online_weight()
    );
}

#[test]
fn vote_republish() {
    let mut system = System::new();
    let node1 = system.make_node();
    let node2 = system.make_node();
    let key2 = PrivateKey::new();
    // by not setting a private key on node1's wallet for genesis account, it is stopped from voting
    let wallet_id = node2.wallets.wallet_ids()[0];
    node2
        .wallets
        .insert_adhoc2(&wallet_id, &key2.private_key(), true)
        .unwrap();

    // send1 and send2 are forks of each other
    let send1 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::nano(1000),
        key2.account().into(),
        &DEV_GENESIS_KEY,
        node1.work_generate_dev(*DEV_GENESIS_HASH),
    ));
    let send2 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::nano(2000),
        key2.account().into(),
        &DEV_GENESIS_KEY,
        node1.work_generate_dev(*DEV_GENESIS_HASH),
    ));

    // process send1 first, this will make sure send1 goes into the ledger and an election is started
    node1.process_active(send1.clone());
    assert_timely_msg(
        Duration::from_secs(5),
        || node2.block_exists(&send1.hash()),
        "block not found on node2",
    );
    assert_timely_msg(
        Duration::from_secs(5),
        || node1.active.active(&send1),
        "not active on node 1",
    );
    assert_timely_msg(
        Duration::from_secs(5),
        || node2.active.active(&send1),
        "not active on node 2",
    );

    // now process send2, send2 will not go in the ledger because only the first block of a fork goes in the ledger
    node1.process_active(send2.clone());
    assert_timely_msg(
        Duration::from_secs(5),
        || node1.active.active(&send2),
        "send2 not active on node 2",
    );

    // send2 cannot be synced because it is not in the ledger of node1, it is only in the election object in RAM on node1
    assert_eq!(node1.block_exists(&send2.hash()), false);

    // the vote causes the election to reach quorum and for the vote (and block?) to be published from node1 to node2
    let vote = Arc::new(Vote::new_final(&DEV_GENESIS_KEY, vec![send2.hash()]));
    let channel_id = ChannelId::from(999);
    node1
        .vote_processor_queue
        .vote(vote, channel_id, VoteSource::Live);

    // FIXME: there is a race condition here, if the vote arrives before the block then the vote is wasted and the test fails
    // we could resend the vote but then there is a race condition between the vote resending and the election reaching quorum on node1
    // the proper fix would be to observe on node2 that both the block and the vote arrived in whatever order
    // the real node will do a confirm request if it needs to find a lost vote

    // check that send2 won on both nodes
    assert_timely_msg(
        Duration::from_secs(5),
        || node1.blocks_confirmed(&[send2.clone()]),
        "not confirmed on node1",
    );
    assert_timely_msg(
        Duration::from_secs(5),
        || node2.blocks_confirmed(&[send2.clone()]),
        "not confirmed on node2",
    );

    // check that send1 is deleted from the ledger on nodes
    assert_eq!(node1.block_exists(&send1.hash()), false);
    assert_eq!(node2.block_exists(&send1.hash()), false);
    assert_timely_eq(
        Duration::from_secs(5),
        || node1.balance(&key2.account()),
        Amount::nano(2000),
    );
    assert_timely_eq(
        Duration::from_secs(5),
        || node2.balance(&key2.account()),
        Amount::nano(2000),
    );
}

// This test places block send1 onto every node. Then it creates block send2 (which is a fork of send1) and sends it to node1.
// Then it sends a vote for send2 to node1 and expects node2 to also get the block plus vote and confirm send2.
// TODO: This test enforces the order block followed by vote on node1, should vote followed by block also work? It doesn't currently.
#[test]
fn vote_by_hash_republish() {
    let mut system = System::new();
    let node1 = system.make_node();
    let node2 = system.make_node();
    let key2 = PrivateKey::new();
    // by not setting a private key on node1's wallet for genesis account, it is stopped from voting
    let wallet_id = node2.wallets.wallet_ids()[0];
    node2
        .wallets
        .insert_adhoc2(&wallet_id, &key2.private_key(), true)
        .unwrap();

    // send1 and send2 are forks of each other
    let send1 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::nano(1000),
        key2.account().into(),
        &DEV_GENESIS_KEY,
        node1.work_generate_dev(*DEV_GENESIS_HASH),
    ));
    let send2 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::nano(2000),
        key2.account().into(),
        &DEV_GENESIS_KEY,
        node1.work_generate_dev(*DEV_GENESIS_HASH),
    ));

    // give block send1 to node1 and check that an election for send1 starts on both nodes
    node1.process_active(send1.clone());
    assert_timely_msg(
        Duration::from_secs(5),
        || node1.active.active(&send1),
        "not active on node 1",
    );
    assert_timely_msg(
        Duration::from_secs(5),
        || node2.active.active(&send1),
        "not active on node 2",
    );

    // give block send2 to node1 and wait until the block is received and processed by node1
    node1.network_filter.clear_all();
    node1.process_active(send2.clone());
    assert_timely_msg(
        Duration::from_secs(5),
        || node1.active.active(&send2),
        "send2 not active on node 1",
    );

    // construct a vote for send2 in order to overturn send1
    let vote = Arc::new(Vote::new_final(&DEV_GENESIS_KEY, vec![send2.hash()]));
    node1
        .vote_processor_queue
        .vote(vote, ChannelId::from(999), VoteSource::Live);

    // send2 should win on both nodes
    assert_timely_msg(
        Duration::from_secs(5),
        || node1.blocks_confirmed(&[send2.clone()]),
        "not confirmed on node1",
    );
    assert_timely_msg(
        Duration::from_secs(5),
        || node2.blocks_confirmed(&[send2.clone()]),
        "not confirmed on node2",
    );
    assert_eq!(node1.block_exists(&send1.hash()), false);
    assert_eq!(node2.block_exists(&send1.hash()), false);
}

#[test]
fn fork_election_invalid_block_signature() {
    let mut system = System::new();
    let node1 = system.make_node();

    // send1 and send2 are forks of each other
    let send1 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::nano(1000),
        (*DEV_GENESIS_ACCOUNT).into(),
        &DEV_GENESIS_KEY,
        node1.work_generate_dev(*DEV_GENESIS_HASH),
    ));
    let send2 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::nano(2000),
        (*DEV_GENESIS_ACCOUNT).into(),
        &DEV_GENESIS_KEY,
        node1.work_generate_dev(*DEV_GENESIS_HASH),
    ));
    let mut send3 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::nano(2000),
        (*DEV_GENESIS_ACCOUNT).into(),
        &DEV_GENESIS_KEY,
        node1.work_generate_dev(*DEV_GENESIS_HASH),
    ));
    send3.set_block_signature(&Signature::new()); // Invalid signature

    let channel = make_fake_channel(&node1);
    node1.inbound_message_queue.put(
        Message::Publish(Publish::new_forward(send1.clone())),
        channel.info.clone(),
    );
    assert_timely_msg(
        Duration::from_secs(5),
        || node1.active.active(&send1),
        "not active on node 1",
    );
    let election = node1.active.election(&send1.qualified_root()).unwrap();
    assert_eq!(1, election.mutex.lock().unwrap().last_blocks.len());

    node1.inbound_message_queue.put(
        Message::Publish(Publish::new_forward(send3)),
        channel.info.clone(),
    );
    node1.inbound_message_queue.put(
        Message::Publish(Publish::new_forward(send2.clone())),
        channel.info.clone(),
    );
    assert_timely_msg(
        Duration::from_secs(3),
        || election.mutex.lock().unwrap().last_blocks.len() > 1,
        "block len was < 2",
    );
    assert_eq!(
        election
            .mutex
            .lock()
            .unwrap()
            .last_blocks
            .get(&send2.hash())
            .unwrap()
            .block_signature(),
        send2.block_signature()
    );
}

#[test]
fn confirm_back() {
    let mut system = System::new();
    let node = system.make_node();
    let key = PrivateKey::new();

    let send1 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::raw(1),
        key.account().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(*DEV_GENESIS_HASH),
    ));
    let open = Block::State(StateBlock::new(
        key.account(),
        BlockHash::zero(),
        key.public_key(),
        Amount::raw(1),
        send1.hash().into(),
        &key,
        node.work_generate_dev(&key),
    ));
    let send2 = Block::State(StateBlock::new(
        key.account(),
        open.hash(),
        key.public_key(),
        Amount::zero(),
        (*DEV_GENESIS_ACCOUNT).into(),
        &key,
        node.work_generate_dev(open.hash()),
    ));

    node.process_active(send1.clone());
    node.process_active(open.clone());
    node.process_active(send2.clone());

    assert_timely_msg(
        Duration::from_secs(5),
        || node.block_exists(&send2.hash()),
        "send2 not found",
    );

    start_election(&node, &send1.hash());
    start_election(&node, &open.hash());
    start_election(&node, &send2.hash());
    assert_eq!(node.active.len(), 3);
    let vote = Arc::new(Vote::new_final(&DEV_GENESIS_KEY, vec![send2.hash()]));
    node.vote_processor_queue
        .vote(vote, ChannelId::from(999), VoteSource::Live);
    assert_timely_eq(Duration::from_secs(10), || node.active.len(), 0);
}

#[test]
fn rollback_vote_self() {
    let mut system = System::new();
    let mut flags = NodeFlags::default();
    flags.disable_request_loop = true;
    let node = system.build_node().flags(flags).finish();
    let wallet_id = node.wallets.wallet_ids()[0];
    let key = PrivateKey::new();

    // send half the voting weight to a non voting rep to ensure quorum cannot be reached
    let send1 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::MAX / 2,
        key.account().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(*DEV_GENESIS_HASH),
    ));

    let open = Block::State(StateBlock::new(
        key.account(),
        BlockHash::zero(),
        key.public_key(),
        Amount::MAX / 2,
        send1.hash().into(),
        &key,
        node.work_generate_dev(&key),
    ));

    // send 1 raw
    let send2 = Block::State(StateBlock::new(
        key.account(),
        open.hash(),
        key.public_key(),
        open.balance_field().unwrap() - Amount::raw(1),
        (*DEV_GENESIS_ACCOUNT).into(),
        &key,
        node.work_generate_dev(open.hash()),
    ));

    // fork of send2 block
    let fork = Block::State(StateBlock::new(
        key.account(),
        open.hash(),
        key.public_key(),
        open.balance_field().unwrap() - Amount::raw(2),
        (*DEV_GENESIS_ACCOUNT).into(),
        &key,
        node.work_generate_dev(open.hash()),
    ));

    // Process and mark the first 2 blocks as confirmed to allow voting
    node.process(send1.clone()).unwrap();
    node.process(open.clone()).unwrap();
    node.confirm(open.hash());

    // wait until the rep weights have caught up with the weight transfer
    assert_timely_eq(
        Duration::from_secs(5),
        || node.ledger.weight(&key.public_key()),
        Amount::MAX / 2,
    );

    // process forked blocks, send2 will be the winner because it was first and there are no votes yet
    node.process_active(send2.clone());
    assert_timely_msg(
        Duration::from_secs(5),
        || node.active.election(&send2.qualified_root()).is_some(),
        "election not found",
    );
    let election = node.active.election(&send2.qualified_root()).unwrap();
    node.process_active(fork.clone());
    assert_timely_eq(
        Duration::from_secs(5),
        || election.mutex.lock().unwrap().last_blocks.len(),
        2,
    );
    assert_eq!(election.winner_hash().unwrap(), send2.hash());

    {
        // The write guard prevents the block processor from performing the rollback
        let _write_guard = node.ledger.write_queue.wait(Writer::Testing);

        assert_eq!(0, node.active.votes_with_weight(&election).len());
        // Vote with key to switch the winner
        node.active.vote_applier.vote(
            &election,
            &key.public_key(),
            0,
            &fork.hash(),
            VoteSource::Live,
        );
        assert_eq!(1, node.active.votes_with_weight(&election).len());
        // The winner changed
        assert_eq!(election.winner_hash().unwrap(), fork.hash(),);

        // Insert genesis key in the wallet
        node.wallets
            .insert_adhoc2(&wallet_id, &DEV_GENESIS_KEY.private_key(), true)
            .unwrap();

        // Without the rollback being finished, the aggregator should not reply with any vote
        let channel = make_fake_channel(&node);

        node.request_aggregator
            .request(vec![(send2.hash(), send2.root())], channel.channel_id());

        assert_always_eq(
            Duration::from_secs(1),
            || {
                node.stats.count(
                    StatType::RequestAggregatorReplies,
                    DetailType::NormalVote,
                    Direction::Out,
                )
            },
            0,
        );

        // Going out of the scope allows the rollback to complete
    }

    // A vote is eventually generated from the local representative
    let is_genesis_vote = |info: &&VoteWithWeightInfo| info.representative == *DEV_GENESIS_PUB_KEY;

    assert_timely_eq(
        Duration::from_secs(5),
        || node.active.votes_with_weight(&election).len(),
        2,
    );
    let votes_with_weight = node.active.votes_with_weight(&election);
    assert_eq!(1, votes_with_weight.iter().filter(is_genesis_vote).count());
    let vote = votes_with_weight.iter().find(is_genesis_vote).unwrap();
    assert_eq!(fork.hash(), vote.hash);
}

// Test that rep_crawler removes unreachable reps from its search results.
// This test creates three principal representatives (rep1, rep2, genesis_rep) and
// one node for searching them (searching_node).
#[test]
fn rep_crawler_rep_remove() {
    let mut system = System::new();
    let searching_node = system.make_node(); // will be used to find principal representatives
    let keys_rep1 = PrivateKey::new(); // Principal representative 1
    let keys_rep2 = PrivateKey::new(); // Principal representative 2

    let min_pr_weight = searching_node
        .online_reps
        .lock()
        .unwrap()
        .minimum_principal_weight();

    // Send enough nanos to Rep1 to make it a principal representative
    let send_to_rep1 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - (min_pr_weight * 2),
        keys_rep1.account().into(),
        &DEV_GENESIS_KEY,
        system
            .work
            .generate_dev2((*DEV_GENESIS_HASH).into())
            .unwrap(),
    ));

    // Receive by Rep1
    let receive_rep1 = Block::State(StateBlock::new(
        keys_rep1.account(),
        BlockHash::zero(),
        keys_rep1.public_key(),
        min_pr_weight * 2,
        send_to_rep1.hash().into(),
        &keys_rep1,
        system
            .work
            .generate_dev2(keys_rep1.public_key().into())
            .unwrap(),
    ));

    // Send enough nanos to Rep2 to make it a principal representative
    let send_to_rep2 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        send_to_rep1.hash(),
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - (min_pr_weight * 4),
        keys_rep2.account().into(),
        &DEV_GENESIS_KEY,
        system
            .work
            .generate_dev2(send_to_rep1.hash().into())
            .unwrap(),
    ));

    // Receive by Rep2
    let receive_rep2 = Block::State(StateBlock::new(
        keys_rep2.account(),
        BlockHash::zero(),
        keys_rep2.public_key(),
        min_pr_weight * 2,
        send_to_rep2.hash().into(),
        &keys_rep2,
        system
            .work
            .generate_dev2(keys_rep2.public_key().into())
            .unwrap(),
    ));

    searching_node.process(send_to_rep1).unwrap();
    searching_node.process(receive_rep1).unwrap();
    searching_node.process(send_to_rep2).unwrap();
    searching_node.process(receive_rep2).unwrap();

    // Create channel for Rep1
    let channel_rep1 = make_fake_channel(&searching_node);

    // Ensure Rep1 is found by the rep_crawler after receiving a vote from it
    let vote_rep1 = Arc::new(Vote::new(&keys_rep1, 0, 0, vec![*DEV_GENESIS_HASH]));
    searching_node
        .rep_crawler
        .force_process(vote_rep1, channel_rep1.channel_id());
    assert_timely_eq(
        Duration::from_secs(5),
        || {
            searching_node
                .online_reps
                .lock()
                .unwrap()
                .peered_reps_count()
        },
        1,
    );

    let reps = searching_node.online_reps.lock().unwrap().peered_reps();
    assert_eq!(1, reps.len());
    assert_eq!(
        min_pr_weight * 2,
        searching_node.ledger.weight(&reps[0].account)
    );
    assert_eq!(keys_rep1.public_key(), reps[0].account);
    assert_eq!(channel_rep1.channel_id(), reps[0].channel_id);

    // When rep1 disconnects then rep1 should not be found anymore
    channel_rep1.info.close();
    assert_timely_eq(
        Duration::from_secs(5),
        || {
            searching_node
                .online_reps
                .lock()
                .unwrap()
                .peered_reps_count()
        },
        0,
    );

    // Add working node for genesis representative
    let node_genesis_rep = system.make_node();
    let wallet_id = node_genesis_rep.wallets.wallet_ids()[0];
    node_genesis_rep
        .wallets
        .insert_adhoc2(&wallet_id, &DEV_GENESIS_KEY.private_key(), true)
        .unwrap();
    let channel_genesis_rep = searching_node
        .network_info
        .read()
        .unwrap()
        .find_node_id(&node_genesis_rep.get_node_id())
        .unwrap()
        .clone();

    // genesis_rep should be found as principal representative after receiving a vote from it
    let vote_genesis_rep = Arc::new(Vote::new(&DEV_GENESIS_KEY, 0, 0, vec![*DEV_GENESIS_HASH]));
    searching_node
        .rep_crawler
        .force_process(vote_genesis_rep, channel_genesis_rep.channel_id());
    assert_timely_eq(
        Duration::from_secs(10),
        || {
            searching_node
                .online_reps
                .lock()
                .unwrap()
                .peered_reps_count()
        },
        1,
    );

    // Start a node for Rep2 and wait until it is connected
    let node_rep2 = system.make_node();
    searching_node
        .peer_connector
        .connect_to(node_rep2.tcp_listener.local_address());
    assert_timely_msg(
        Duration::from_secs(10),
        || {
            searching_node
                .network_info
                .read()
                .unwrap()
                .find_node_id(&node_rep2.get_node_id())
                .is_some()
        },
        "channel to rep2 not found",
    );
    let channel_rep2 = searching_node
        .network_info
        .read()
        .unwrap()
        .find_node_id(&node_rep2.get_node_id())
        .unwrap()
        .clone();

    // Rep2 should be found as a principal representative after receiving a vote from it
    let vote_rep2 = Arc::new(Vote::new(&keys_rep2, 0, 0, vec![*DEV_GENESIS_HASH]));
    searching_node
        .rep_crawler
        .force_process(vote_rep2, channel_rep2.channel_id());
    assert_timely_eq(
        Duration::from_secs(10),
        || {
            searching_node
                .online_reps
                .lock()
                .unwrap()
                .peered_reps_count()
        },
        2,
    );

    // TODO rewrite this test and the missing part below this commit
    // ... part missing:
}

#[test]
fn epoch_conflict_confirm() {
    let mut system = System::new();
    let config0 = System::default_config_without_backlog_population();
    let node0 = system.build_node().config(config0).finish();

    let config1 = System::default_config_without_backlog_population();
    let node1 = system.build_node().config(config1).finish();

    let key = PrivateKey::new();
    let epoch_signer = DEV_GENESIS_KEY.clone();

    let send = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::raw(1),
        key.account().into(),
        &DEV_GENESIS_KEY,
        system
            .work
            .generate_dev2((*DEV_GENESIS_HASH).into())
            .unwrap(),
    ));

    let open = Block::State(StateBlock::new(
        key.account(),
        BlockHash::zero(),
        key.public_key(),
        Amount::raw(1),
        send.hash().into(),
        &key,
        system.work.generate_dev2(key.public_key().into()).unwrap(),
    ));

    let change = Block::State(StateBlock::new(
        key.account(),
        open.hash(),
        key.public_key(),
        Amount::raw(1),
        Link::zero(),
        &key,
        system.work.generate_dev2(open.hash().into()).unwrap(),
    ));

    let send2 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        send.hash(),
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::raw(2),
        open.hash().into(),
        &DEV_GENESIS_KEY,
        system.work.generate_dev2(send.hash().into()).unwrap(),
    ));

    let epoch_open = Block::State(StateBlock::new(
        change.root().into(),
        BlockHash::zero(),
        PublicKey::zero(),
        Amount::zero(),
        node0.ledger.epoch_link(Epoch::Epoch1).unwrap(),
        &epoch_signer,
        system.work.generate_dev2(open.hash().into()).unwrap(),
    ));

    // Process initial blocks on node1
    node1.process(send.clone()).unwrap();
    node1.process(send2.clone()).unwrap();
    node1.process(open.clone()).unwrap();

    // Confirm open block in node1 to allow generating votes
    node1.confirm(open.hash());

    // Process initial blocks on node0
    node0.process(send.clone()).unwrap();
    node0.process(send2.clone()).unwrap();
    node0.process(open.clone()).unwrap();

    // Process conflicting blocks on node 0 as blocks coming from live network
    node0.process_active(change.clone());
    node0.process_active(epoch_open.clone());

    // Ensure blocks were propagated to both nodes
    assert_timely(Duration::from_secs(5), || {
        node0.blocks_exist(&[change.clone(), epoch_open.clone()])
    });
    assert_timely(Duration::from_secs(5), || {
        node1.blocks_exist(&[change.clone(), epoch_open.clone()])
    });

    // Confirm initial blocks in node1 to allow generating votes later
    start_elections(
        &node1,
        &[change.hash(), epoch_open.hash(), send2.hash()],
        true,
    );
    assert_timely(Duration::from_secs(5), || {
        node1.blocks_confirmed(&[change.clone(), epoch_open.clone(), send2.clone()])
    });

    // Start elections for node0 for conflicting change and epoch_open blocks (those two blocks have the same root)
    activate_hashes(&node0, &[change.hash(), epoch_open.hash()]);
    assert_timely(Duration::from_secs(5), || {
        node0.vote_router.active(&change.hash()) && node0.vote_router.active(&epoch_open.hash())
    });

    // Make node1 a representative
    let wallet_id = node1.wallets.wallet_ids()[0];
    node1
        .wallets
        .insert_adhoc2(&wallet_id, &DEV_GENESIS_KEY.private_key(), true)
        .unwrap();

    // Ensure both conflicting blocks were successfully processed and confirmed
    assert_timely(Duration::from_secs(15), || {
        node0.blocks_confirmed(&[change.clone(), epoch_open.clone()])
    });
}

#[test]
fn node_receive_quorum() {
    let mut system = System::new();
    let node1 = system.make_node();

    let wallet_id = node1.wallets.wallet_ids()[0];
    let key = PrivateKey::new();
    let previous = node1.latest(&DEV_GENESIS_ACCOUNT);

    node1
        .wallets
        .insert_adhoc2(&wallet_id, &key.private_key(), true)
        .unwrap();

    let send = Block::LegacySend(SendBlock::new(
        &previous,
        &key.account(),
        &(node1.ledger.constants.genesis_amount - Amount::nano(1000)),
        &DEV_GENESIS_KEY,
        system.work.generate_dev2(previous.into()).unwrap(),
    ));

    node1.process_active(send.clone());

    assert_timely_msg(
        Duration::from_secs(10),
        || node1.block_exists(&send.hash()),
        "send block not found",
    );

    assert_timely_msg(
        Duration::from_secs(10),
        || node1.active.election(&send.qualified_root()).is_some(),
        "election not found",
    );

    let election = node1.active.election(&send.qualified_root()).unwrap();
    assert!(!node1.active.confirmed(&election));
    assert_eq!(1, election.mutex.lock().unwrap().last_votes.len());

    let node2 = system.make_disconnected_node();
    let wallet_id2 = node2.wallets.wallet_ids()[0];

    node2
        .wallets
        .insert_adhoc2(&wallet_id2, &DEV_GENESIS_KEY.private_key(), true)
        .unwrap();
    assert!(node1.balance(&key.account()).is_zero());

    node2
        .peer_connector
        .connect_to(node1.tcp_listener.local_address());

    assert_timely_msg(
        Duration::from_secs(10),
        || !node1.balance(&key.account()).is_zero(),
        "balance is still zero",
    );
}

#[test]
fn auto_bootstrap() {
    let mut system = System::new();
    let config = System::default_config_without_backlog_population();
    let mut node_flags = NodeFlags::default();
    node_flags.disable_bootstrap_bulk_push_client = true;
    node_flags.disable_lazy_bootstrap = true;

    let node0 = system
        .build_node()
        .config(config.clone())
        .flags(node_flags.clone())
        .finish();
    let wallet_id = node0.wallets.wallet_ids()[0];
    let key2 = PrivateKey::new();

    node0
        .wallets
        .insert_adhoc2(&wallet_id, &DEV_GENESIS_KEY.private_key(), true)
        .unwrap();
    node0
        .wallets
        .insert_adhoc2(&wallet_id, &key2.private_key(), true)
        .unwrap();

    let send1 = node0
        .wallets
        .send_action2(
            &wallet_id,
            *DEV_GENESIS_ACCOUNT,
            key2.account(),
            node0.config.receive_minimum,
            0,
            true,
            None,
        )
        .unwrap();

    assert_timely_msg(
        Duration::from_secs(10),
        || node0.balance(&key2.account()) == node0.config.receive_minimum,
        "balance not updated",
    );

    let node1 = system.make_node();

    establish_tcp(&node1, &node0);

    assert_timely_msg(
        Duration::from_secs(10),
        || node1.balance(&key2.account()) == node0.config.receive_minimum,
        "balance not synced",
    );

    assert!(node1.block_exists(&send1.hash()));

    // Wait for block receive
    assert_timely_msg(
        Duration::from_secs(5),
        || node1.ledger.block_count() == 3,
        "block count not 3",
    );

    // Confirmation for all blocks
    assert_timely_msg(
        Duration::from_secs(5),
        || node1.ledger.cemented_count() == 3,
        "cemented count not 3",
    );
}

#[test]
fn fork_open_flip() {
    let mut system = System::new();
    let node1 = system.make_node();
    let wallet_id = node1.wallets.wallet_ids()[0];

    let key1 = PrivateKey::new();
    let rep1 = PrivateKey::new();
    let rep2 = PrivateKey::new();

    // send 1 raw from genesis to key1 on both node1 and node2
    let send1 = Block::LegacySend(SendBlock::new(
        &DEV_GENESIS_HASH,
        &key1.account(),
        &(Amount::MAX - Amount::raw(1)),
        &DEV_GENESIS_KEY,
        system
            .work
            .generate_dev2((*DEV_GENESIS_HASH).into())
            .unwrap(),
    ));
    node1.process_active(send1.clone());

    // We should be keeping this block
    let open1 = Block::LegacyOpen(OpenBlock::new(
        send1.hash(),
        rep1.public_key(),
        key1.account(),
        &key1,
        system.work.generate_dev2(key1.public_key().into()).unwrap(),
    ));

    // create a fork of block open1, this block will lose the election
    let open2 = Block::LegacyOpen(OpenBlock::new(
        send1.hash(),
        rep2.public_key(),
        key1.account(),
        &key1,
        system.work.generate_dev2(key1.public_key().into()).unwrap(),
    ));
    assert_ne!(open1.hash(), open2.hash());

    // give block open1 to node1, manually trigger an election for open1 and ensure it is in the ledger
    let open1 = node1.process(open1).unwrap();
    node1.election_schedulers.manual.push(open1.clone(), None);
    assert_timely_msg(
        Duration::from_secs(5),
        || node1.active.election(&open1.qualified_root()).is_some(),
        "election for open1 not found",
    );
    let election = node1.active.election(&open1.qualified_root()).unwrap();
    election.transition_active();

    // create node2, with blocks send1 and open2 pre-initialised in the ledger,
    // so that block open1 cannot possibly get in the ledger before open2 via background sync
    system.initialization_blocks.push(send1.clone());
    system.initialization_blocks.push(open2.clone());
    let node2 = system.make_node();
    system.initialization_blocks.clear();
    let open2 = node2.block(&open2.hash()).unwrap();

    // ensure open2 is in node2 ledger (and therefore has sideband) and manually trigger an election for open2
    assert_timely_msg(
        Duration::from_secs(5),
        || node2.block_exists(&open2.hash()),
        "open2 not found on node2",
    );
    node2.election_schedulers.manual.push(open2.clone(), None);
    assert_timely_msg(
        Duration::from_secs(5),
        || node2.active.election(&open2.qualified_root()).is_some(),
        "election for open2 not found",
    );
    let election2 = node2.active.election(&open2.qualified_root()).unwrap();
    election2.transition_active();

    assert_timely_eq(Duration::from_secs(5), || node1.active.len(), 2);
    assert_timely_eq(Duration::from_secs(5), || node2.active.len(), 2);

    // allow node1 to vote and wait for open1 to be confirmed on node1
    node1
        .wallets
        .insert_adhoc2(&wallet_id, &DEV_GENESIS_KEY.private_key(), true)
        .unwrap();
    assert_timely_msg(
        Duration::from_secs(5),
        || node1.block_confirmed(&open1.hash()),
        "open1 not confirmed on node1",
    );

    // Notify both nodes of both blocks, both nodes will become aware that a fork exists
    node1.process_active(open2.clone().into());
    node2.process_active(open1.clone().into());

    assert_timely_eq(Duration::from_secs(5), || election.vote_count(), 2); // one more than expected due to elections having dummy votes

    // Node2 should eventually settle on open1
    assert_timely_msg(
        Duration::from_secs(10),
        || node2.block_exists(&open1.hash()),
        "open1 not found on node2",
    );
    assert_timely_msg(
        Duration::from_secs(5),
        || node1.block_confirmed(&open1.hash()),
        "open1 not confirmed on node1",
    );
    let election_status = election.mutex.lock().unwrap().status.clone();
    assert_eq!(open1.hash(), election_status.winner.unwrap().hash());
    assert_eq!(Amount::MAX - Amount::raw(1), election_status.tally);

    // check the correct blocks are in the ledgers
    assert!(node1.block_exists(&open1.hash()));
    assert!(node2.block_exists(&&open1.hash()));
    assert!(!node2.block_exists(&open2.hash()));
}

#[test]
fn unconfirmed_send() {
    let mut system = System::new();

    let node1 = system.make_node();
    let wallet_id1 = node1.wallets.wallet_ids()[0];
    node1
        .wallets
        .insert_adhoc2(&wallet_id1, &DEV_GENESIS_KEY.private_key(), true)
        .unwrap();

    let key2 = PrivateKey::new();
    let node2 = system.make_node();
    let wallet_id2 = node2.wallets.wallet_ids()[0];
    node2
        .wallets
        .insert_adhoc2(&wallet_id2, &key2.private_key(), true)
        .unwrap();

    // firstly, send two units from node1 to node2 and expect that both nodes see the block as confirmed
    // (node1 will start an election for it, vote on it and node2 gets synced up)
    let send1 = node1
        .wallets
        .send_action2(
            &wallet_id1,
            *DEV_GENESIS_ACCOUNT,
            key2.account(),
            Amount::nano(2),
            0,
            true,
            None,
        )
        .unwrap();

    assert_timely_msg(
        Duration::from_secs(5),
        || node1.block_confirmed(&send1.hash()),
        "send1 not confirmed on node1",
    );

    assert_timely_msg(
        Duration::from_secs(5),
        || node2.block_confirmed(&send1.hash()),
        "send1 not confirmed on node2",
    );

    // wait until receive1 (auto-receive created by wallet) is cemented
    assert_timely_eq(
        Duration::from_secs(5),
        || {
            let tx = node2.store.tx_begin_read();
            node2
                .store
                .confirmation_height
                .get(&tx, &key2.account())
                .unwrap_or_default()
                .height
        },
        1,
    );

    assert_eq!(node2.balance(&key2.account()), Amount::nano(2));

    let recv1 = {
        let tx = node2.store.tx_begin_read();
        node2
            .ledger
            .find_receive_block_by_send_hash(&tx, &key2.account(), &send1.hash())
            .unwrap()
    };

    // create send2 to send from node2 to node1 and save it to node2's ledger without triggering an election (node1 does not hear about it)
    let send2 = Block::State(StateBlock::new(
        key2.account(),
        recv1.hash(),
        *DEV_GENESIS_PUB_KEY,
        Amount::nano(1),
        (*DEV_GENESIS_ACCOUNT).into(),
        &key2,
        system.work.generate_dev2(recv1.hash().into()).unwrap(),
    ));
    assert_eq!(
        BlockStatus::Progress,
        node2.process_local(send2.clone()).unwrap()
    );

    let send3 = node2
        .wallets
        .send_action2(
            &wallet_id2,
            key2.account(),
            *DEV_GENESIS_ACCOUNT,
            Amount::nano(1),
            0,
            true,
            None,
        )
        .unwrap();
    assert_timely_msg(
        Duration::from_secs(5),
        || node2.block_confirmed(&send2.hash()),
        "send2 not confirmed on node2",
    );
    assert_timely_msg(
        Duration::from_secs(5),
        || node1.block_confirmed(&send2.hash()),
        "send2 not confirmed on node1",
    );
    assert_timely_msg(
        Duration::from_secs(5),
        || node2.block_confirmed(&send3.hash()),
        "send3 not confirmed on node2",
    );
    assert_timely_msg(
        Duration::from_secs(5),
        || node1.block_confirmed(&send3.hash()),
        "send3 not confirmed on node1",
    );
    assert_timely_eq(Duration::from_secs(5), || node2.ledger.cemented_count(), 7);
    assert_timely_eq(
        Duration::from_secs(5),
        || node1.balance(&DEV_GENESIS_ACCOUNT),
        Amount::MAX,
    );
}

#[test]
fn block_processor_signatures() {
    let mut system = System::new();
    let node = system.make_node();

    // Insert the genesis key into the wallet for signing operations
    let wallet_id = node.wallets.wallet_ids()[0];
    node.wallets
        .insert_adhoc2(&wallet_id, &DEV_GENESIS_KEY.private_key(), true)
        .unwrap();

    let latest = node.latest(&*DEV_GENESIS_ACCOUNT);
    let key1 = PrivateKey::new();
    let key2 = PrivateKey::new();
    let key3 = PrivateKey::new();

    // Create a valid send block
    let send1 = BlockBuilder::state()
        .account(*DEV_GENESIS_ACCOUNT)
        .previous(latest)
        .representative(*DEV_GENESIS_PUB_KEY)
        .balance(node.ledger.constants.genesis_amount - Amount::nano(1000))
        .link(key1.account())
        .sign(&DEV_GENESIS_KEY)
        .work(node.work_generate_dev(latest))
        .build();

    // Create additional send blocks with proper signatures
    let send2 = BlockBuilder::state()
        .account(*DEV_GENESIS_ACCOUNT)
        .previous(send1.hash())
        .representative(*DEV_GENESIS_PUB_KEY)
        .balance(node.ledger.constants.genesis_amount - Amount::nano(2000))
        .link(key2.account())
        .sign(&DEV_GENESIS_KEY)
        .work(node.work_generate_dev(send1.hash()))
        .build();

    let send3 = BlockBuilder::state()
        .account(*DEV_GENESIS_ACCOUNT)
        .previous(send2.hash())
        .representative(*DEV_GENESIS_PUB_KEY)
        .balance(node.ledger.constants.genesis_amount - Amount::nano(3000))
        .link(key3.account())
        .sign(&DEV_GENESIS_KEY)
        .work(node.work_generate_dev(send2.hash()))
        .build();

    // Create a block with an invalid signature (tampered signature bits)
    let mut send4 = BlockBuilder::state()
        .account(*DEV_GENESIS_ACCOUNT)
        .previous(send3.hash())
        .representative(*DEV_GENESIS_PUB_KEY)
        .balance(node.ledger.constants.genesis_amount - Amount::nano(4000))
        .link(key3.account())
        .sign(&DEV_GENESIS_KEY)
        .work(node.work_generate_dev(send3.hash()))
        .build();

    // Tamper with the signature
    send4.set_block_signature(&Signature::new());

    // Invalid signature bit (force)
    let mut send5 = BlockBuilder::state()
        .account(*DEV_GENESIS_ACCOUNT)
        .previous(send3.hash())
        .representative(*DEV_GENESIS_PUB_KEY)
        .balance(node.ledger.constants.genesis_amount - Amount::nano(5000))
        .link(key3.account())
        .sign(&DEV_GENESIS_KEY)
        .work(node.work_generate_dev(send3.hash()))
        .build();

    send5.set_block_signature(&Signature::new());
    // Invalid signature to unchecked
    node.unchecked
        .put(send5.previous().into(), UncheckedInfo::new(send5.clone()));

    // Create a valid receive block
    let receive1 = BlockBuilder::state()
        .account(key1.account())
        .previous(BlockHash::zero())
        .representative(*DEV_GENESIS_PUB_KEY) // No previous block for the account (open block)
        .balance(Amount::nano(1000))
        .link(send1.hash())
        .sign(&key1)
        .work(node.work_generate_dev(key1.account()))
        .build();

    let receive2 = BlockBuilder::state()
        .account(key2.account())
        .previous(BlockHash::zero())
        .representative(*DEV_GENESIS_PUB_KEY) // No previous block for the account (open block)
        .balance(Amount::nano(1000))
        .link(send2.hash())
        .sign(&key2)
        .work(node.work_generate_dev(key2.account()))
        .build();

    // Invalid private key
    let receive3 = BlockBuilder::state()
        .account(key3.account())
        .previous(BlockHash::zero())
        .representative(*DEV_GENESIS_PUB_KEY) // No previous block for the account (open block)
        .balance(Amount::nano(1000))
        .link(send3.hash())
        .sign(&key2)
        .work(node.work_generate_dev(key3.account()))
        .build();

    node.process_active(send1.clone());
    node.process_active(send2.clone());
    node.process_active(send3.clone());
    node.process_active(send4.clone());
    node.process_active(receive1.clone());
    node.process_active(receive2.clone());
    node.process_active(receive3.clone());

    assert_timely(Duration::from_secs(5), || {
        node.block_exists(&receive2.hash())
    });

    assert_timely_eq(Duration::from_secs(5), || node.unchecked.len(), 0);

    assert!(node.block(&receive3.hash()).is_none()); // Invalid signer
    assert!(node.block(&send4.hash()).is_none()); // Invalid signature via process_active
    assert!(node.block(&send5.hash()).is_none()); // Invalid signature via unchecked
}

#[test]
fn block_confirm() {
    let mut system = System::new();
    let node1 = system.make_node();
    let node2 = system.make_node();
    let wallet_id2 = node2.wallets.wallet_ids()[0];
    let key = PrivateKey::new();

    let send1 = BlockBuilder::state()
        .account(*DEV_GENESIS_ACCOUNT)
        .previous(*DEV_GENESIS_HASH)
        .representative(*DEV_GENESIS_ACCOUNT)
        .balance(node1.ledger.constants.genesis_amount - Amount::nano(1000))
        .link(key.account())
        .sign(&DEV_GENESIS_KEY)
        .work(node1.work_generate_dev(*DEV_GENESIS_HASH))
        .build();

    let hash1 = send1.hash();

    assert_eq!(
        node1
            .block_processor
            .add(send1.clone().into(), BlockSource::Live, ChannelId::LOOPBACK),
        true
    );
    assert_eq!(
        node2
            .block_processor
            .add(send1.clone().into(), BlockSource::Live, ChannelId::LOOPBACK,),
        true
    );

    assert_timely(Duration::from_secs(5), || {
        node1
            .ledger
            .any()
            .block_exists_or_pruned(&node1.store.tx_begin_read(), &hash1)
            && node2
                .ledger
                .any()
                .block_exists_or_pruned(&node2.store.tx_begin_read(), &hash1)
    });

    assert!(node1
        .ledger
        .any()
        .block_exists_or_pruned(&node1.ledger.read_txn(), &hash1));
    assert!(node2
        .ledger
        .any()
        .block_exists_or_pruned(&node2.ledger.read_txn(), &hash1));

    // Confirm send1 on node2 so it can vote for send2
    start_election(&node2, &hash1);

    assert_timely_eq(
        Duration::from_secs(5),
        || node2.active.election(&send1.qualified_root()).is_some() as u64,
        1,
    );

    // Make node2 genesis representative so it can vote
    node2
        .wallets
        .insert_adhoc2(&wallet_id2, &DEV_GENESIS_KEY.private_key(), true)
        .unwrap();

    assert_timely_eq(
        Duration::from_secs(10),
        || node1.active.recently_cemented_list().len(),
        1,
    );
}

/// Confirm a complex dependency graph. Uses frontiers confirmation which will fail to
/// confirm a frontier optimistically then fallback to pessimistic confirmation.
#[test]
fn dependency_graph_frontier() {
    let mut system = System::new();
    let node1 = system
        .build_node()
        .config(System::default_config_without_backlog_population())
        .finish();
    let node2 = system.make_node();
    let key1 = PrivateKey::new();
    let key2 = PrivateKey::new();
    let key3 = PrivateKey::new();

    // Send to key1
    let gen_send1 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::raw(1),
        key1.account().into(),
        &DEV_GENESIS_KEY,
        node1.work_generate_dev(*DEV_GENESIS_HASH),
    ));

    // Receive from genesis
    let key1_open = Block::State(StateBlock::new(
        key1.account(),
        BlockHash::zero(),
        key1.public_key(),
        Amount::raw(1),
        gen_send1.hash().into(),
        &key1,
        node1.work_generate_dev(&key1),
    ));
    // Send to genesis
    let key1_send1 = Block::State(StateBlock::new(
        key1.account(),
        key1_open.hash(),
        key1.public_key(),
        Amount::zero(),
        (*DEV_GENESIS_ACCOUNT).into(),
        &key1,
        node1.work_generate_dev(key1_open.hash()),
    ));
    // Receive from key1
    let gen_receive = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        gen_send1.hash(),
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX,
        key1_send1.hash().into(),
        &DEV_GENESIS_KEY,
        node1.work_generate_dev(gen_send1.hash()),
    ));
    // Send to key2
    let gen_send2 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        gen_receive.hash(),
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::raw(2),
        key2.account().into(),
        &DEV_GENESIS_KEY,
        node1.work_generate_dev(gen_receive.hash()),
    ));
    // Receive from genesis
    let key2_open = Block::State(StateBlock::new(
        key2.account(),
        BlockHash::zero(),
        key2.public_key(),
        Amount::raw(2),
        gen_send2.hash().into(),
        &key2,
        node1.work_generate_dev(&key2),
    ));
    // Send to key3
    let key2_send1 = Block::State(StateBlock::new(
        key2.account(),
        key2_open.hash(),
        key2.public_key(),
        Amount::raw(1),
        key3.account().into(),
        &key2,
        node1.work_generate_dev(key2_open.hash()),
    ));
    // Receive from key2
    let key3_open = Block::State(StateBlock::new(
        key3.account(),
        BlockHash::zero(),
        key3.public_key(),
        Amount::raw(1),
        key2_send1.hash().into(),
        &key3,
        node1.work_generate_dev(&key3),
    ));
    // Send to key1
    let key2_send2 = Block::State(StateBlock::new(
        key2.account(),
        key2_send1.hash(),
        key2.public_key(),
        Amount::zero(),
        key1.account().into(),
        &key2,
        node1.work_generate_dev(key2_send1.hash()),
    ));
    // Receive from key2
    let key1_receive = Block::State(StateBlock::new(
        key1.account(),
        key1_send1.hash(),
        key1.public_key(),
        Amount::raw(1),
        key2_send2.hash().into(),
        &key1,
        node1.work_generate_dev(key1_send1.hash()),
    ));
    // Send to key3
    let key1_send2 = Block::State(StateBlock::new(
        key1.account(),
        key1_receive.hash(),
        key1.public_key(),
        Amount::zero(),
        key3.account().into(),
        &key1,
        node1.work_generate_dev(key1_receive.hash()),
    ));
    // Receive from key1
    let key3_receive = Block::State(StateBlock::new(
        key3.account(),
        key3_open.hash(),
        key3.public_key(),
        Amount::raw(2),
        key1_send2.hash().into(),
        &key3,
        node1.work_generate_dev(key3_open.hash()),
    ));
    // Upgrade key3
    let key3_epoch = Block::State(StateBlock::new(
        key3.account(),
        key3_receive.hash(),
        key3.public_key(),
        Amount::raw(2),
        node1.ledger.epoch_link(Epoch::Epoch1).unwrap(),
        &DEV_GENESIS_KEY,
        node1.work_generate_dev(key3_receive.hash()),
    ));

    for node in &system.nodes {
        node.process_multi(&[
            gen_send1.clone(),
            key1_open.clone(),
            key1_send1.clone(),
            gen_receive.clone(),
            gen_send2.clone(),
            key2_open.clone(),
            key2_send1.clone(),
            key3_open.clone(),
            key2_send2.clone(),
            key1_receive.clone(),
            key1_send2.clone(),
            key3_receive.clone(),
            key3_epoch.clone(),
        ]);
    }

    // node1 can vote, but only on the first block
    node1.insert_into_wallet(&DEV_GENESIS_KEY);
    assert_timely(Duration::from_secs(10), || {
        node2.active.active_root(&gen_send1.qualified_root())
    });
    start_election(&node1, &gen_send1.hash());
    assert_timely_eq(
        Duration::from_secs(15),
        || node1.ledger.cemented_count(),
        node1.ledger.block_count(),
    );
    assert_timely_eq(
        Duration::from_secs(15),
        || node2.ledger.cemented_count(),
        node2.ledger.block_count(),
    );
}

/// Confirm a complex dependency graph starting from the first block
#[test]
fn dependency_graph() {
    let mut system = System::new();
    let node = system
        .build_node()
        .config(System::default_config_without_backlog_population())
        .finish();
    let key1 = PrivateKey::new();
    let key2 = PrivateKey::new();
    let key3 = PrivateKey::new();

    // Send to key1
    let gen_send1 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::raw(1),
        key1.account().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(*DEV_GENESIS_HASH),
    ));

    // Receive from genesis
    let key1_open = Block::State(StateBlock::new(
        key1.account(),
        BlockHash::zero(),
        key1.public_key(),
        Amount::raw(1),
        gen_send1.hash().into(),
        &key1,
        node.work_generate_dev(&key1),
    ));
    // Send to genesis
    let key1_send1 = Block::State(StateBlock::new(
        key1.account(),
        key1_open.hash(),
        key1.public_key(),
        Amount::zero(),
        (*DEV_GENESIS_ACCOUNT).into(),
        &key1,
        node.work_generate_dev(key1_open.hash()),
    ));
    // Receive from key1
    let gen_receive = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        gen_send1.hash(),
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX,
        key1_send1.hash().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(gen_send1.hash()),
    ));
    // Send to key2
    let gen_send2 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        gen_receive.hash(),
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::raw(2),
        key2.account().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(gen_receive.hash()),
    ));
    // Receive from genesis
    let key2_open = Block::State(StateBlock::new(
        key2.account(),
        BlockHash::zero(),
        key2.public_key(),
        Amount::raw(2),
        gen_send2.hash().into(),
        &key2,
        node.work_generate_dev(&key2),
    ));
    // Send to key3
    let key2_send1 = Block::State(StateBlock::new(
        key2.account(),
        key2_open.hash(),
        key2.public_key(),
        Amount::raw(1),
        key3.account().into(),
        &key2,
        node.work_generate_dev(key2_open.hash()),
    ));
    // Receive from key2
    let key3_open = Block::State(StateBlock::new(
        key3.account(),
        BlockHash::zero(),
        key3.public_key(),
        Amount::raw(1),
        key2_send1.hash().into(),
        &key3,
        node.work_generate_dev(&key3),
    ));
    // Send to key1
    let key2_send2 = Block::State(StateBlock::new(
        key2.account(),
        key2_send1.hash(),
        key2.public_key(),
        Amount::zero(),
        key1.account().into(),
        &key2,
        node.work_generate_dev(key2_send1.hash()),
    ));
    // Receive from key2
    let key1_receive = Block::State(StateBlock::new(
        key1.account(),
        key1_send1.hash(),
        key1.public_key(),
        Amount::raw(1),
        key2_send2.hash().into(),
        &key1,
        node.work_generate_dev(key1_send1.hash()),
    ));
    // Send to key3
    let key1_send2 = Block::State(StateBlock::new(
        key1.account(),
        key1_receive.hash(),
        key1.public_key(),
        Amount::zero(),
        key3.account().into(),
        &key1,
        node.work_generate_dev(key1_receive.hash()),
    ));
    // Receive from key1
    let key3_receive = Block::State(StateBlock::new(
        key3.account(),
        key3_open.hash(),
        key3.public_key(),
        Amount::raw(2),
        key1_send2.hash().into(),
        &key3,
        node.work_generate_dev(key3_open.hash()),
    ));
    // Upgrade key3
    let key3_epoch = Block::State(StateBlock::new(
        key3.account(),
        key3_receive.hash(),
        key3.public_key(),
        Amount::raw(2),
        node.ledger.epoch_link(Epoch::Epoch1).unwrap(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(key3_receive.hash()),
    ));

    for node in &system.nodes {
        node.process_multi(&[
            gen_send1.clone(),
            key1_open.clone(),
            key1_send1.clone(),
            gen_receive.clone(),
            gen_send2.clone(),
            key2_open.clone(),
            key2_send1.clone(),
            key3_open.clone(),
            key2_send2.clone(),
            key1_receive.clone(),
            key1_send2.clone(),
            key3_receive.clone(),
            key3_epoch.clone(),
        ]);
    }

    // Hash -> Ancestors
    let dependency_graph: HashMap<BlockHash, Vec<BlockHash>> = [
        (key1_open.hash(), vec![gen_send1.hash()]),
        (key1_send1.hash(), vec![key1_open.hash()]),
        (gen_receive.hash(), vec![gen_send1.hash(), key1_open.hash()]),
        (gen_send2.hash(), vec![gen_receive.hash()]),
        (key2_open.hash(), vec![gen_send2.hash()]),
        (key2_send1.hash(), vec![key2_open.hash()]),
        (key3_open.hash(), vec![key2_send1.hash()]),
        (key2_send2.hash(), vec![key2_send1.hash()]),
        (
            key1_receive.hash(),
            vec![key1_send1.hash(), key2_send2.hash()],
        ),
        (key1_send2.hash(), vec![key1_send1.hash()]),
        (
            key3_receive.hash(),
            vec![key3_open.hash(), key1_send2.hash()],
        ),
        (key3_epoch.hash(), vec![key3_receive.hash()]),
    ]
    .into();
    assert_eq!(node.ledger.block_count() - 2, dependency_graph.len() as u64);

    // Start an election for the first block of the dependency graph, and ensure all blocks are eventually confirmed
    node.insert_into_wallet(&DEV_GENESIS_KEY);
    start_election(&node, &gen_send1.hash());
    assert_timely(Duration::from_secs(15), || {
        // Not many blocks should be active simultaneously
        assert!(node.active.len() < 6);

        // Ensure that active blocks have their ancestors confirmed
        let error = dependency_graph.iter().any(|entry| {
            if node.vote_router.active(entry.0) {
                for ancestor in entry.1 {
                    if !node.block_confirmed(ancestor) {
                        return true;
                    }
                }
            }
            false
        });
        assert!(!error);
        error || node.ledger.cemented_count() == node.ledger.block_count()
    });
    assert_eq!(node.ledger.cemented_count(), node.ledger.block_count());
    assert_timely(Duration::from_secs(5), || node.active.len() == 0);
}

#[test]
fn fork_keep() {
    let mut system = System::new();
    let node1 = system.make_node();
    let node2 = system.make_node();
    let key1 = PrivateKey::new();
    let key2 = PrivateKey::new();
    // send1 and send2 fork to different accounts
    let send1 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::raw(100),
        key1.account().into(),
        &DEV_GENESIS_KEY,
        node1.work_generate_dev(*DEV_GENESIS_HASH),
    ));
    let send2 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::raw(100),
        key2.account().into(),
        &DEV_GENESIS_KEY,
        node1.work_generate_dev(*DEV_GENESIS_HASH),
    ));
    node1.process_active(send1.clone());
    node2.process_active(send1.clone());
    assert_timely_eq(Duration::from_secs(5), || node1.active.len(), 1);
    assert_timely_eq(Duration::from_secs(5), || node2.active.len(), 1);
    node1.insert_into_wallet(&DEV_GENESIS_KEY);
    // Fill node with forked blocks
    node1.process_active(send2.clone());
    assert_timely(Duration::from_secs(5), || node1.active.active(&send2));
    node2.process_active(send2.clone());
    let election1 = node2
        .active
        .election(&QualifiedRoot::new(
            (*DEV_GENESIS_HASH).into(),
            *DEV_GENESIS_HASH,
        ))
        .unwrap();
    assert_eq!(election1.vote_count(), 1);
    assert!(node1.block_exists(&send1.hash()));
    assert!(node2.block_exists(&send1.hash()));
    // Wait until the genesis rep makes a vote
    assert_timely(Duration::from_secs(60), || election1.vote_count() != 1);
    // The vote should be in agreement with what we already have.
    let guard = election1.mutex.lock().unwrap();
    let (winner_hash, winner_tally) = guard.last_tally.iter().next().unwrap();
    assert_eq!(*winner_hash, send1.hash());
    assert_eq!(*winner_tally, Amount::MAX - Amount::raw(100));
    assert!(node1.block_exists(&send1.hash()));
    assert!(node2.block_exists(&send1.hash()));
}
