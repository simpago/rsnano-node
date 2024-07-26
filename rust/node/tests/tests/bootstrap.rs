use std::{sync::atomic::Ordering, thread::sleep, time::Duration};

use rsnano_core::{
    Account, Amount, BlockEnum, BlockHash, KeyPair, StateBlock, UncheckedKey, WalletId,
    DEV_GENESIS_KEY,
};
use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH};
use rsnano_node::{
    bootstrap::{BootstrapInitiatorExt, BootstrapStrategy},
    config::{FrontiersConfirmationMode, NodeFlags},
    node::NodeExt,
    wallets::WalletsExt,
};

use crate::tests::helpers::get_available_port;

use super::helpers::{assert_timely, assert_timely_eq, establish_tcp, System};

#[test]
fn bootstrap_processor_lazy_hash() {
    let mut system = System::new();
    let mut config = System::default_config();
    config.frontiers_confirmation = FrontiersConfirmationMode::Disabled;
    let mut flags = NodeFlags::new();
    flags.disable_bootstrap_bulk_push_client = true;
    let node0 = system.build_node().config(config).flags(flags).finish();

    let key1 = KeyPair::new();
    let key2 = KeyPair::new();

    let send1 = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_ACCOUNT,
        Amount::MAX - Amount::nano(1000),
        key1.public_key().into(),
        &DEV_GENESIS_KEY,
        node0.work_generate_dev((*DEV_GENESIS_HASH).into()),
    ));

    let receive1 = BlockEnum::State(StateBlock::new(
        key1.public_key(),
        BlockHash::zero(),
        key1.public_key(),
        Amount::nano(1000),
        send1.hash().into(),
        &key1,
        node0.work_generate_dev(key1.public_key().into()),
    ));

    let send2 = BlockEnum::State(StateBlock::new(
        key1.public_key(),
        receive1.hash(),
        key1.public_key(),
        Amount::zero(),
        key2.public_key().into(),
        &key1,
        node0.work_generate_dev(receive1.hash().into()),
    ));

    let receive2 = BlockEnum::State(StateBlock::new(
        key2.public_key(),
        BlockHash::zero(),
        key2.public_key(),
        Amount::nano(1000),
        send2.hash().into(),
        &key2,
        node0.work_generate_dev(key2.public_key().into()),
    ));

    // Processing test chain
    let blocks = [send1, receive1, send2, receive2.clone()];
    node0.process_multi(&blocks);

    assert_timely(
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
        assert_eq!(lazy.attempt.id, receive2.hash().to_string());
    }

    // Check processed blocks
    assert_timely_eq(
        Duration::from_secs(10),
        || node1.balance(&key2.public_key()),
        Amount::nano(1000),
    );
}

#[test]
fn bootstrap_processor_lazy_hash_bootstrap_id() {
    let mut system = System::new();
    let mut config = System::default_config();
    config.frontiers_confirmation = FrontiersConfirmationMode::Disabled;
    let mut flags = NodeFlags::new();
    flags.disable_bootstrap_bulk_push_client = true;
    let node0 = system.build_node().config(config).flags(flags).finish();

    let key1 = KeyPair::new();
    let key2 = KeyPair::new();
    // Generating test chain

    let send1 = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_ACCOUNT,
        Amount::MAX - Amount::nano(1000),
        key1.public_key().into(),
        &DEV_GENESIS_KEY,
        node0.work_generate_dev((*DEV_GENESIS_HASH).into()),
    ));

    let receive1 = BlockEnum::State(StateBlock::new(
        key1.public_key(),
        BlockHash::zero(),
        key1.public_key(),
        Amount::nano(1000),
        send1.hash().into(),
        &key1,
        node0.work_generate_dev(key1.public_key().into()),
    ));

    let send2 = BlockEnum::State(StateBlock::new(
        key1.public_key(),
        receive1.hash(),
        key1.public_key(),
        Amount::zero(),
        key2.public_key().into(),
        &key1,
        node0.work_generate_dev(receive1.hash().into()),
    ));

    let receive2 = BlockEnum::State(StateBlock::new(
        key2.public_key(),
        BlockHash::zero(),
        key2.public_key(),
        Amount::nano(1000),
        send2.hash().into(),
        &key2,
        node0.work_generate_dev(key2.public_key().into()),
    ));

    // Processing test chain
    let blocks = [send1, receive1, send2, receive2.clone()];
    node0.process_multi(&blocks);

    assert_timely(
        Duration::from_secs(5),
        || node0.blocks_exist(&blocks),
        "blocks not processed",
    );

    // Start lazy bootstrap with last block in chain known
    let node1 = system.make_disconnected_node();
    establish_tcp(&node1, &node0);
    node1
        .bootstrap_initiator
        .bootstrap_lazy(receive2.hash().into(), true, "123456".to_string());

    {
        let lazy_attempt = node1
            .bootstrap_initiator
            .current_lazy_attempt()
            .expect("no lazy attempt found");

        let BootstrapStrategy::Lazy(lazy) = lazy_attempt.as_ref() else {
            panic!("not lazy")
        };
        assert_eq!(lazy.attempt.id, "123456".to_string());
    }

    // Check processed blocks
    assert_timely_eq(
        Duration::from_secs(10),
        || node1.balance(&key2.public_key()),
        Amount::nano(1000),
    );
}

#[test]
fn bootstrap_processor_lazy_pruning_missing_block() {
    let mut system = System::new();
    let mut config = System::default_config();
    config.frontiers_confirmation = FrontiersConfirmationMode::Disabled;
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

    let key1 = KeyPair::new();
    let key2 = KeyPair::new();

    // send from genesis to key1
    let send1 = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_ACCOUNT,
        Amount::MAX - Amount::nano(1000),
        key1.public_key().into(),
        &DEV_GENESIS_KEY,
        node1.work_generate_dev((*DEV_GENESIS_HASH).into()),
    ));

    // send from genesis to key2
    let send2 = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        send1.hash(),
        *DEV_GENESIS_ACCOUNT,
        Amount::MAX - Amount::nano(2000),
        key2.public_key().into(),
        &DEV_GENESIS_KEY,
        node1.work_generate_dev(send1.hash().into()),
    ));

    // open account key1
    let receive1 = BlockEnum::State(StateBlock::new(
        key1.public_key(),
        BlockHash::zero(),
        key1.public_key(),
        Amount::nano(1000),
        send1.hash().into(),
        &key1,
        node1.work_generate_dev(key1.public_key().into()),
    ));

    //  open account key2
    let receive2 = BlockEnum::State(StateBlock::new(
        key2.public_key(),
        BlockHash::zero(),
        key2.public_key(),
        Amount::nano(1000),
        send2.hash().into(),
        &key2,
        node1.work_generate_dev(key2.public_key().into()),
    ));

    // add the blocks without starting elections because elections publish blocks
    // and the publishing would interefere with the testing
    let blocks = [send1.clone(), send2.clone(), receive1, receive2];
    node1.process_multi(&blocks);

    assert_timely(
        Duration::from_secs(5),
        || node1.blocks_exist(&blocks),
        "blocks not processed",
    );

    node1.confirm_multi(&blocks);

    assert_timely(
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

    assert_timely(
        Duration::from_secs(5),
        || {
            lazy_attempt.attempt().stopped()
                || lazy_attempt.attempt().requeued_pulls.load(Ordering::SeqCst) >= 4
        },
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
    let mut config = System::default_config();
    config.frontiers_confirmation = FrontiersConfirmationMode::Disabled;

    let mut flags = NodeFlags::new();
    flags.disable_bootstrap_bulk_push_client = true;

    let node0 = system
        .build_node()
        .config(config.clone())
        .flags(flags.clone())
        .finish();

    let key1 = KeyPair::new();

    let send1 = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_ACCOUNT,
        Amount::MAX - Amount::nano(1000),
        key1.public_key().into(),
        &DEV_GENESIS_KEY,
        node0.work_generate_dev((*DEV_GENESIS_HASH).into()),
    ));

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
    assert_timely(
        Duration::from_secs(10),
        || !node1.bootstrap_initiator.in_progress(),
        "attempt not cancelled",
    );
}

#[test]
fn bootstrap_processor_multiple_attempts() {
    let mut system = System::new();
    let mut config = System::default_config();
    config.frontiers_confirmation = FrontiersConfirmationMode::Disabled;
    let mut flags = NodeFlags::new();
    flags.disable_bootstrap_bulk_push_client = true;
    let node0 = system.build_node().config(config).flags(flags).finish();

    let key1 = KeyPair::new();
    let key2 = KeyPair::new();
    // Generating test chain

    let send1 = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_ACCOUNT,
        Amount::MAX - Amount::nano(1000),
        key1.public_key().into(),
        &DEV_GENESIS_KEY,
        node0.work_generate_dev((*DEV_GENESIS_HASH).into()),
    ));

    let receive1 = BlockEnum::State(StateBlock::new(
        key1.public_key(),
        BlockHash::zero(),
        key1.public_key(),
        Amount::nano(1000),
        send1.hash().into(),
        &key1,
        node0.work_generate_dev(key1.public_key().into()),
    ));

    let send2 = BlockEnum::State(StateBlock::new(
        key1.public_key(),
        receive1.hash(),
        key1.public_key(),
        Amount::zero(),
        key2.public_key().into(),
        &key1,
        node0.work_generate_dev(receive1.hash().into()),
    ));

    let receive2 = BlockEnum::State(StateBlock::new(
        key2.public_key(),
        BlockHash::zero(),
        key2.public_key(),
        Amount::nano(1000),
        send2.hash().into(),
        &key2,
        node0.work_generate_dev(key2.public_key().into()),
    ));

    // Processing test chain
    let blocks = [send1, receive1, send2, receive2.clone()];
    node0.process_multi(&blocks);

    assert_timely(
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

    assert_timely(
        Duration::from_secs(5),
        || node1.bootstrap_initiator.current_legacy_attempt().is_some(),
        "no legacy attempt found",
    );

    // Check processed blocks
    assert_timely(
        Duration::from_secs(10),
        || node1.balance(&key2.public_key()) > Amount::zero(),
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
    let mut config = System::default_config();
    config.frontiers_confirmation = FrontiersConfirmationMode::Disabled;
    let mut flags = NodeFlags::new();
    flags.disable_bootstrap_bulk_push_client = true;
    flags.disable_legacy_bootstrap = true;
    flags.disable_ascending_bootstrap = true;
    flags.disable_ongoing_bootstrap = true;
    let node0 = system.build_node().config(config).flags(flags).finish();

    let key1 = KeyPair::new();
    let key2 = KeyPair::new();
    // Generating test chain

    let send1 = BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_ACCOUNT,
        Amount::MAX - Amount::nano(1000),
        key1.public_key().into(),
        &DEV_GENESIS_KEY,
        node0.work_generate_dev((*DEV_GENESIS_HASH).into()),
    ));

    let receive1 = BlockEnum::State(StateBlock::new(
        key1.public_key(),
        BlockHash::zero(),
        key1.public_key(),
        Amount::nano(1000),
        send1.hash().into(),
        &key1,
        node0.work_generate_dev(key1.public_key().into()),
    ));

    let send2 = BlockEnum::State(StateBlock::new(
        key1.public_key(),
        receive1.hash(),
        key1.public_key(),
        Amount::zero(),
        key2.public_key().into(),
        &key1,
        node0.work_generate_dev(receive1.hash().into()),
    ));

    let receive2 = BlockEnum::State(StateBlock::new(
        key2.public_key(),
        BlockHash::zero(),
        key2.public_key(),
        Amount::nano(1000),
        send2.hash().into(),
        &key2,
        node0.work_generate_dev(key2.public_key().into()),
    ));

    // Processing test chain
    let blocks = [send1, receive1, send2, receive2.clone()];
    node0.process_multi(&blocks);

    assert_timely(
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
        .insert_adhoc2(&wallet_id, &key2.private_key(), true)
        .unwrap();
    node1.bootstrap_wallet();
    {
        node1
            .bootstrap_initiator
            .current_wallet_attempt()
            .expect("no wallet attempt found");
    }
    // Check processed blocks
    assert_timely(
        Duration::from_secs(10),
        || node1.block_exists(&receive2.hash()),
        "receive 2 not  found",
    )
}
