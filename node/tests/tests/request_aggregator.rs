use rsnano_core::{Amount, Block, BlockHash, KeyPair, StateBlock, DEV_GENESIS_KEY};
use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH, DEV_GENESIS_PUB_KEY};
use rsnano_messages::ConfirmAck;
use rsnano_node::{
    config::NodeFlags,
    stats::{DetailType, Direction, StatType},
    wallets::WalletsExt,
};
use std::{sync::Arc, time::Duration};
use test_helpers::{assert_timely_eq, assert_timely_msg, make_fake_channel, System};

#[test]
fn one() {
    let mut system = System::new();
    let config = System::default_config_without_backlog_population();
    let node = system.build_node().config(config).finish();
    node.wallets
        .insert_adhoc2(
            &node.wallets.wallet_ids()[0],
            &DEV_GENESIS_KEY.private_key(),
            true,
        )
        .unwrap();

    let mut send1 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::nano(1000),
        (*DEV_GENESIS_ACCOUNT).into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(*DEV_GENESIS_HASH),
    ));

    let request = vec![(send1.hash(), send1.root())];

    let channel = make_fake_channel(&node);

    node.request_aggregator
        .request(request.clone(), channel.channel_id());
    assert_timely_msg(
        Duration::from_secs(3),
        || node.request_aggregator.is_empty(),
        "aggregator not empty",
    );
    assert_timely_eq(
        Duration::from_secs(3),
        || {
            node.stats.count(
                StatType::Requests,
                DetailType::RequestsUnknown,
                Direction::In,
            )
        },
        1,
    );

    // Process and confirm
    node.ledger
        .process(&mut node.ledger.rw_txn(), &mut send1)
        .unwrap();
    node.confirm(send1.hash());

    let channel = make_fake_channel(&node);
    // In the ledger but no vote generated yet
    node.request_aggregator
        .request(request.clone(), channel.channel_id());
    assert_timely_msg(
        Duration::from_secs(3),
        || node.request_aggregator.is_empty(),
        "aggregator not empty",
    );
    assert_timely_msg(
        Duration::from_secs(3),
        || {
            node.stats.count(
                StatType::Requests,
                DetailType::RequestsGeneratedVotes,
                Direction::In,
            ) > 0
        },
        "no votes generated",
    );

    // Already cached
    // TODO: This is outdated, aggregator should not be using cache
    let dummy_channel = make_fake_channel(&node);
    node.request_aggregator
        .request(request, dummy_channel.channel_id());
    assert_timely_msg(
        Duration::from_secs(3),
        || node.request_aggregator.is_empty(),
        "aggregator not empty",
    );
    assert_timely_eq(
        Duration::from_secs(3),
        || {
            node.stats.count(
                StatType::Aggregator,
                DetailType::AggregatorAccepted,
                Direction::In,
            )
        },
        3,
    );
    assert_timely_eq(
        Duration::from_secs(3),
        || {
            node.stats.count(
                StatType::Aggregator,
                DetailType::AggregatorDropped,
                Direction::In,
            )
        },
        0,
    );
    assert_timely_eq(
        Duration::from_secs(3),
        || {
            node.stats.count(
                StatType::Requests,
                DetailType::RequestsUnknown,
                Direction::In,
            )
        },
        1,
    );
    assert_timely_eq(
        Duration::from_secs(3),
        || {
            node.stats.count(
                StatType::Requests,
                DetailType::RequestsGeneratedVotes,
                Direction::In,
            )
        },
        2,
    );
    assert_timely_eq(
        Duration::from_secs(3),
        || {
            node.stats.count(
                StatType::Requests,
                DetailType::RequestsCannotVote,
                Direction::In,
            )
        },
        0,
    );
}

#[test]
fn one_update() {
    let mut system = System::new();
    let config = System::default_config_without_backlog_population();
    let node = system.build_node().config(config).finish();
    node.wallets
        .insert_adhoc2(
            &node.wallets.wallet_ids()[0],
            &DEV_GENESIS_KEY.private_key(),
            true,
        )
        .unwrap();

    let key1 = KeyPair::new();

    let send1 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::nano(1000),
        key1.account().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(*DEV_GENESIS_HASH),
    ));
    node.process(send1.clone()).unwrap();
    node.confirm(send1.hash());

    let send2 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        send1.hash(),
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::nano(2000),
        (*DEV_GENESIS_ACCOUNT).into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(send1.hash()),
    ));
    node.process(send2.clone()).unwrap();
    node.confirm(send2.hash());

    let receive1 = Block::State(StateBlock::new(
        key1.account(),
        BlockHash::zero(),
        *DEV_GENESIS_PUB_KEY,
        Amount::nano(1000),
        send1.hash().into(),
        &key1,
        node.work_generate_dev(&key1),
    ));
    node.process(receive1.clone()).unwrap();
    node.confirm(receive1.hash());

    let dummy_channel = make_fake_channel(&node);

    let request1 = vec![(send2.hash(), send2.root())];
    node.request_aggregator
        .request(request1, dummy_channel.channel_id());

    // Update the pool of requests with another hash
    let request2 = vec![(receive1.hash(), receive1.root())];
    node.request_aggregator
        .request(request2, dummy_channel.channel_id());

    // In the ledger but no vote generated yet
    assert_timely_msg(
        Duration::from_secs(3),
        || {
            node.stats.count(
                StatType::Requests,
                DetailType::RequestsGeneratedVotes,
                Direction::In,
            ) > 0
        },
        "generated votes",
    );
    assert_timely_msg(
        Duration::from_secs(3),
        || node.request_aggregator.is_empty(),
        "aggregator empty",
    );
    assert_timely_eq(
        Duration::from_secs(3),
        || {
            node.stats.count(
                StatType::Aggregator,
                DetailType::AggregatorAccepted,
                Direction::In,
            )
        },
        2,
    );
    assert_timely_eq(
        Duration::from_secs(3),
        || {
            node.stats.count(
                StatType::Requests,
                DetailType::RequestsGeneratedHashes,
                Direction::In,
            )
        },
        2,
    );
    assert_eq!(
        node.stats.count(
            StatType::Aggregator,
            DetailType::AggregatorDropped,
            Direction::In,
        ),
        0
    );
    assert_eq!(
        node.stats.count(
            StatType::Requests,
            DetailType::RequestsUnknown,
            Direction::In,
        ),
        0
    );
    assert_eq!(
        node.stats.count(
            StatType::Requests,
            DetailType::RequestsCachedHashes,
            Direction::In,
        ),
        0
    );
    assert_eq!(
        node.stats.count(
            StatType::Requests,
            DetailType::RequestsCachedVotes,
            Direction::In,
        ),
        0
    );
    assert_eq!(
        node.stats.count(
            StatType::Requests,
            DetailType::RequestsCannotVote,
            Direction::In,
        ),
        0
    );
}

#[test]
fn two() {
    let mut system = System::new();
    let config = System::default_config_without_backlog_population();
    let node = system.build_node().config(config).finish();
    node.wallets
        .insert_adhoc2(
            &node.wallets.wallet_ids()[0],
            &DEV_GENESIS_KEY.private_key(),
            true,
        )
        .unwrap();

    let key1 = KeyPair::new();

    let send1 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::raw(1),
        key1.account().into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(*DEV_GENESIS_HASH),
    ));
    node.process(send1.clone()).unwrap();
    node.confirm(send1.hash());

    let send2 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        send1.hash(),
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::raw(2),
        (*DEV_GENESIS_ACCOUNT).into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(send1.hash()),
    ));
    node.process(send2.clone()).unwrap();
    node.confirm(send2.hash());

    let receive1 = Block::State(StateBlock::new(
        key1.account(),
        BlockHash::zero(),
        *DEV_GENESIS_PUB_KEY,
        Amount::raw(1),
        send1.hash().into(),
        &key1,
        node.work_generate_dev(&key1),
    ));
    node.process(receive1.clone()).unwrap();
    node.confirm(receive1.hash());

    let request = vec![
        (send2.hash(), send2.root()),
        (receive1.hash(), receive1.root()),
    ];
    let dummy_channel = make_fake_channel(&node);

    // Process both blocks
    node.request_aggregator
        .request(request.clone(), dummy_channel.channel_id());
    // One vote should be generated for both blocks
    assert_timely_msg(
        Duration::from_secs(3),
        || {
            node.stats.count(
                StatType::Requests,
                DetailType::RequestsGeneratedVotes,
                Direction::In,
            ) > 0
        },
        "generated votes",
    );
    assert_timely_msg(
        Duration::from_secs(3),
        || node.request_aggregator.is_empty(),
        "aggregator empty",
    );
    // The same request should now send the cached vote
    node.request_aggregator
        .request(request.clone(), dummy_channel.channel_id());
    assert_timely_msg(
        Duration::from_secs(3),
        || node.request_aggregator.is_empty(),
        "aggregator empty",
    );
    assert_eq!(
        node.stats.count(
            StatType::Aggregator,
            DetailType::AggregatorAccepted,
            Direction::In,
        ),
        2
    );
    assert_eq!(
        node.stats.count(
            StatType::Aggregator,
            DetailType::AggregatorDropped,
            Direction::In,
        ),
        0
    );
    assert_timely_eq(
        Duration::from_secs(3),
        || {
            node.stats.count(
                StatType::Requests,
                DetailType::RequestsUnknown,
                Direction::In,
            )
        },
        0,
    );
    assert_timely_eq(
        Duration::from_secs(3),
        || {
            node.stats.count(
                StatType::Requests,
                DetailType::RequestsGeneratedHashes,
                Direction::In,
            )
        },
        4,
    );
    assert_timely_eq(
        Duration::from_secs(3),
        || {
            node.stats.count(
                StatType::Requests,
                DetailType::RequestsGeneratedVotes,
                Direction::In,
            )
        },
        2,
    );
    assert_timely_eq(
        Duration::from_secs(3),
        || {
            node.stats.count(
                StatType::Requests,
                DetailType::RequestsCannotVote,
                Direction::In,
            )
        },
        0,
    );
    // Make sure the cached vote is for both hashes
    let vote1 = node.history.votes(&send2.root(), &send2.hash(), false);
    let vote2 = node
        .history
        .votes(&receive1.root(), &receive1.hash(), false);
    assert_eq!(vote1.len(), 1);
    assert_eq!(vote2.len(), 1);
    assert!(Arc::ptr_eq(&vote1[0], &vote2[0]));
}

#[test]
fn split() {
    const MAX_VBH: usize = ConfirmAck::HASHES_MAX;
    let mut system = System::new();
    let config = System::default_config_without_backlog_population();
    let node = system.build_node().config(config).finish();
    node.wallets
        .insert_adhoc2(
            &node.wallets.wallet_ids()[0],
            &DEV_GENESIS_KEY.private_key(),
            true,
        )
        .unwrap();

    let mut request = Vec::new();
    let mut blocks = Vec::new();
    let mut previous = *DEV_GENESIS_HASH;

    for i in 0..=MAX_VBH {
        let block = Block::State(StateBlock::new(
            *DEV_GENESIS_ACCOUNT,
            previous,
            *DEV_GENESIS_PUB_KEY,
            Amount::MAX - Amount::raw(i as u128 + 1),
            (*DEV_GENESIS_ACCOUNT).into(),
            &DEV_GENESIS_KEY,
            node.work_generate_dev(previous),
        ));
        previous = block.hash();
        node.process(block.clone()).unwrap();
        request.push((block.hash(), block.root()));
        blocks.push(block);
    }
    // Confirm all blocks
    node.confirm(blocks.last().unwrap().hash());
    assert_eq!(node.ledger.cemented_count(), MAX_VBH as u64 + 2);
    assert_eq!(MAX_VBH + 1, request.len());
    let dummy_channel = make_fake_channel(&node);
    node.request_aggregator
        .request(request, dummy_channel.channel_id());
    // In the ledger but no vote generated yet
    assert_timely_eq(
        Duration::from_secs(3),
        || {
            node.stats.count(
                StatType::Requests,
                DetailType::RequestsGeneratedVotes,
                Direction::In,
            )
        },
        2,
    );
    assert!(node.request_aggregator.is_empty());
    // Two votes were sent, the first one for 12 hashes and the second one for 1 hash
    assert_eq!(
        node.stats.count(
            StatType::Aggregator,
            DetailType::AggregatorAccepted,
            Direction::In,
        ),
        1
    );
    assert_eq!(
        node.stats.count(
            StatType::Aggregator,
            DetailType::AggregatorDropped,
            Direction::In,
        ),
        0
    );
    assert_timely_eq(
        Duration::from_secs(3),
        || {
            node.stats.count(
                StatType::Requests,
                DetailType::RequestsGeneratedHashes,
                Direction::In,
            )
        },
        255 + 1,
    );
    assert_timely_eq(
        Duration::from_secs(3),
        || {
            node.stats.count(
                StatType::Requests,
                DetailType::RequestsGeneratedVotes,
                Direction::In,
            )
        },
        2,
    );
}

#[test]
fn channel_max_queue() {
    let mut system = System::new();
    let mut config = System::default_config_without_backlog_population();
    config.request_aggregator.max_queue = 0;
    let node = system.build_node().config(config).finish();
    node.wallets
        .insert_adhoc2(
            &node.wallets.wallet_ids()[0],
            &DEV_GENESIS_KEY.private_key(),
            true,
        )
        .unwrap();

    let send1 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::nano(1000),
        (*DEV_GENESIS_ACCOUNT).into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(*DEV_GENESIS_HASH),
    ));
    node.process(send1.clone()).unwrap();

    let request = vec![(send1.hash(), send1.root())];
    let channel = make_fake_channel(&node);
    node.request_aggregator
        .request(request.clone(), channel.channel_id());
    node.request_aggregator
        .request(request.clone(), channel.channel_id());

    assert!(
        node.stats.count(
            StatType::Aggregator,
            DetailType::AggregatorDropped,
            Direction::In
        ) > 0
    );
}

#[test]
fn cannot_vote() {
    let mut system = System::new();
    let mut flags = NodeFlags::default();
    flags.disable_request_loop = true;
    let node = system.build_node().flags(flags).finish();

    let send1 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::raw(1),
        (*DEV_GENESIS_ACCOUNT).into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(*DEV_GENESIS_HASH),
    ));

    let send2 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        send1.hash(),
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::raw(2),
        (*DEV_GENESIS_ACCOUNT).into(),
        &DEV_GENESIS_KEY,
        node.work_generate_dev(send1.hash()),
    ));
    node.process(send1.clone()).unwrap();
    node.process(send2.clone()).unwrap();

    node.wallets
        .insert_adhoc2(
            &node.wallets.wallet_ids()[0],
            &DEV_GENESIS_KEY.private_key(),
            true,
        )
        .unwrap();

    assert_eq!(
        node.ledger
            .dependents_confirmed(&node.ledger.read_txn(), &send2),
        false
    );

    // correct + incorrect
    let request = vec![(send2.hash(), send2.root()), (1.into(), send2.root())];
    let dummy_channel = make_fake_channel(&node);
    node.request_aggregator
        .request(request.clone(), dummy_channel.channel_id());

    assert_timely_msg(
        Duration::from_secs(3),
        || node.request_aggregator.is_empty(),
        "aggregator empty",
    );
    assert_eq!(
        node.stats.count(
            StatType::Aggregator,
            DetailType::AggregatorAccepted,
            Direction::In,
        ),
        1
    );
    assert_eq!(
        node.stats.count(
            StatType::Aggregator,
            DetailType::AggregatorDropped,
            Direction::In,
        ),
        0
    );
    assert_timely_eq(
        Duration::from_secs(3),
        || {
            node.stats.count(
                StatType::Requests,
                DetailType::RequestsNonFinal,
                Direction::In,
            )
        },
        2,
    );
    assert_eq!(
        node.stats.count(
            StatType::Requests,
            DetailType::RequestsGeneratedVotes,
            Direction::In,
        ),
        0
    );
    assert_eq!(
        node.stats.count(
            StatType::Requests,
            DetailType::RequestsUnknown,
            Direction::In,
        ),
        0
    );

    // With an ongoing election
    node.election_schedulers.add_manual(send2.clone());
    assert_timely_msg(
        Duration::from_secs(5),
        || node.active.election(&send2.qualified_root()).is_some(),
        "no election",
    );

    node.request_aggregator
        .request(request.clone(), dummy_channel.channel_id());

    assert_timely_msg(
        Duration::from_secs(3),
        || node.request_aggregator.is_empty(),
        "aggregator empty",
    );
    assert_eq!(
        node.stats.count(
            StatType::Aggregator,
            DetailType::AggregatorAccepted,
            Direction::In,
        ),
        2
    );
    assert_eq!(
        node.stats.count(
            StatType::Aggregator,
            DetailType::AggregatorDropped,
            Direction::In,
        ),
        0
    );
    assert_timely_eq(
        Duration::from_secs(3),
        || {
            node.stats.count(
                StatType::Requests,
                DetailType::RequestsNonFinal,
                Direction::In,
            )
        },
        4,
    );
    assert_eq!(
        node.stats.count(
            StatType::Requests,
            DetailType::RequestsGeneratedVotes,
            Direction::In,
        ),
        0
    );
    assert_eq!(
        node.stats.count(
            StatType::Requests,
            DetailType::RequestsUnknown,
            Direction::In,
        ),
        0
    );

    // Confirm send1 and send2
    node.confirm(send1.hash());
    node.confirm(send2.hash());

    node.request_aggregator
        .request(request.clone(), dummy_channel.channel_id());

    assert_timely_msg(
        Duration::from_secs(3),
        || node.request_aggregator.is_empty(),
        "aggregator empty",
    );

    assert_timely_eq(
        Duration::from_secs(3),
        || {
            node.stats.count(
                StatType::Requests,
                DetailType::RequestsGeneratedHashes,
                Direction::In,
            )
        },
        1,
    );
    assert_timely_eq(
        Duration::from_secs(3),
        || {
            node.stats.count(
                StatType::Requests,
                DetailType::RequestsGeneratedVotes,
                Direction::In,
            )
        },
        1,
    );
}
