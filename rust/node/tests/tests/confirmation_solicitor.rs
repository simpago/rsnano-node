use rsnano_core::{Account, Amount, BlockEnum, StateBlock, DEV_GENESIS_KEY};
use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH};
use rsnano_messages::ConfirmReq;
use rsnano_node::{
    config::NodeFlags,
    consensus::{ConfirmationSolicitor, Election, ElectionBehavior, VoteInfo},
    representatives::PeeredRep,
    stats::{DetailType, Direction, StatType},
    transport::ChannelId,
    DEV_NETWORK_PARAMS,
};
use std::sync::Arc;

use super::helpers::{establish_tcp, System};

#[test]
fn batches() {
    let mut system = System::new();
    let mut flags = NodeFlags::default();
    flags.disable_request_loop = true;
    flags.disable_rep_crawler = true;
    let node1 = system.build_node().flags(flags.clone()).finish();
    let node2 = system.build_node().flags(flags).finish();
    let channel1 = establish_tcp(&node2, &node1);
    // Solicitor will only solicit from this representative
    let representative = PeeredRep::new(
        *DEV_GENESIS_ACCOUNT,
        channel1.channel_id(),
        node2.steady_clock.now(),
    );
    let representatives = vec![representative];

    let mut solicitor = ConfirmationSolicitor::new(&DEV_NETWORK_PARAMS, &node2.network);
    solicitor.prepare(&representatives);

    let send = Arc::new(BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_ACCOUNT,
        Amount::MAX - Amount::raw(100),
        Account::from(123).into(),
        &DEV_GENESIS_KEY,
        node1.work_generate_dev((*DEV_GENESIS_HASH).into()),
    )));

    {
        for i in 0..ConfirmReq::HASHES_MAX {
            let election = Election::new(
                i,
                send.clone(),
                ElectionBehavior::Priority,
                Box::new(|_| {}),
                Box::new(|_| {}),
            );

            let data = election.mutex.lock().unwrap();
            assert_eq!(solicitor.add(&election, &data), false);
        }
        // Reached the maximum amount of requests for the channel
        let election = Election::new(
            1000,
            send.clone(),
            ElectionBehavior::Priority,
            Box::new(|_| {}),
            Box::new(|_| {}),
        );
        // Broadcasting should be immediate
        assert_eq!(
            0,
            node2
                .stats
                .count(StatType::Message, DetailType::Publish, Direction::Out)
        );
        let data = election.mutex.lock().unwrap();
        solicitor.broadcast(&data).unwrap();
    }
    // One publish through directed broadcasting and another through random flooding
    assert_eq!(
        2,
        node2
            .stats
            .count(StatType::Message, DetailType::Publish, Direction::Out)
    );
    solicitor.flush();
    assert_eq!(
        1,
        node2
            .stats
            .count(StatType::Message, DetailType::ConfirmReq, Direction::Out)
    );
}

#[test]
fn different_hashes() {
    let mut system = System::new();
    let mut flags = NodeFlags::default();
    flags.disable_request_loop = true;
    flags.disable_rep_crawler = true;
    let node1 = system.build_node().flags(flags.clone()).finish();
    let node2 = system.build_node().flags(flags).finish();
    let channel1 = establish_tcp(&node2, &node1);
    // Solicitor will only solicit from this representative
    let representative = PeeredRep::new(
        *DEV_GENESIS_ACCOUNT,
        channel1.channel_id(),
        node2.steady_clock.now(),
    );
    let representatives = vec![representative];

    let mut solicitor = ConfirmationSolicitor::new(&DEV_NETWORK_PARAMS, &node2.network);
    solicitor.prepare(&representatives);

    let send = Arc::new(BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_ACCOUNT,
        Amount::MAX - Amount::raw(100),
        Account::from(123).into(),
        &DEV_GENESIS_KEY,
        node1.work_generate_dev((*DEV_GENESIS_HASH).into()),
    )));

    let election = Election::new(
        100,
        send.clone(),
        ElectionBehavior::Priority,
        Box::new(|_| {}),
        Box::new(|_| {}),
    );
    let mut data = election.mutex.lock().unwrap();
    // Add a vote for something else, not the winner
    data.last_votes
        .insert(*DEV_GENESIS_ACCOUNT, VoteInfo::new(1, 1.into()));
    // Ensure the request and broadcast goes through
    assert_eq!(solicitor.add(&election, &data), false);
    solicitor.broadcast(&data).unwrap();
    // One publish through directed broadcasting and another through random flooding

    assert_eq!(
        2,
        node2
            .stats
            .count(StatType::Message, DetailType::Publish, Direction::Out)
    );
    solicitor.flush();
    assert_eq!(
        1,
        node2
            .stats
            .count(StatType::Message, DetailType::ConfirmReq, Direction::Out)
    );
}

#[test]
fn bypass_max_requests_cap() {
    let mut system = System::new();
    let mut flags = NodeFlags::default();
    flags.disable_request_loop = true;
    flags.disable_rep_crawler = true;
    let node1 = system.build_node().flags(flags.clone()).finish();
    let node2 = system.build_node().flags(flags).finish();

    let mut solicitor = ConfirmationSolicitor::new(&DEV_NETWORK_PARAMS, &node2.network);

    let mut representatives = Vec::new();
    const MAX_REPRESENTATIVES: usize = 50;
    for i in 0..=MAX_REPRESENTATIVES {
        // Make a temporary channel associated with node2
        let rep = PeeredRep::new(
            Account::from(i as u64),
            ChannelId::from(i),
            node2.steady_clock.now(),
        );
        representatives.push(rep);
    }
    assert_eq!(representatives.len(), MAX_REPRESENTATIVES + 1);
    solicitor.prepare(&representatives);

    let send = Arc::new(BlockEnum::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_ACCOUNT,
        Amount::MAX - Amount::raw(100),
        Account::from(123).into(),
        &DEV_GENESIS_KEY,
        node1.work_generate_dev((*DEV_GENESIS_HASH).into()),
    )));

    let election = Election::new(
        100,
        send.clone(),
        ElectionBehavior::Priority,
        Box::new(|_| {}),
        Box::new(|_| {}),
    );
    let mut data = election.mutex.lock().unwrap();
    // Add a vote for something else, not the winner
    for rep in &representatives {
        data.last_votes
            .insert(rep.account, VoteInfo::new(1, 1.into()));
    }
    // Ensure the request and broadcast goes through
    assert_eq!(solicitor.add(&election, &data), false);
    solicitor.broadcast(&data).unwrap();
    drop(data);
    // All requests went through, the last one would normally not go through due to the cap but a vote for a different hash does not count towards the cap
    // TODO port remainder of test!
}
