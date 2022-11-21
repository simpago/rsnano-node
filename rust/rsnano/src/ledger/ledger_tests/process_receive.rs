use crate::{
    core::{
        Account, Amount, Block, BlockBuilder, BlockEnum, BlockHash, KeyPair, SignatureVerification,
    },
    ledger::{
        ledger_tests::{LedgerWithReceiveBlock, LedgerWithSendBlock},
        ProcessResult, DEV_GENESIS_KEY,
    },
    DEV_CONSTANTS, DEV_GENESIS_ACCOUNT,
};

use super::{LedgerContext, LedgerWithOpenBlock};

#[test]
fn update_sideband() {
    let ctx = LedgerWithReceiveBlock::new();
    let sideband = ctx.receive_block.sideband().unwrap();
    assert_eq!(sideband.account, ctx.receiver_account);
    assert_eq!(sideband.balance, ctx.expected_receiver_balance);
    assert_eq!(sideband.height, 2);
}

#[test]
fn save_block() {
    let ctx = LedgerWithReceiveBlock::new();

    let loaded_block = ctx
        .ledger()
        .store
        .block()
        .get(ctx.txn.txn(), &ctx.receive_block.hash())
        .unwrap();

    let BlockEnum::Receive(loaded_block) = loaded_block else{panic!("not a receive block")};
    assert_eq!(loaded_block, ctx.receive_block);
    assert_eq!(
        loaded_block.sideband().unwrap(),
        ctx.receive_block.sideband().unwrap()
    );
}

#[test]
fn update_block_amount() {
    let ctx = LedgerWithReceiveBlock::new();
    assert_eq!(
        ctx.ledger()
            .amount(ctx.txn.txn(), &ctx.receive_block.hash()),
        Some(Amount::new(25))
    );
}

#[test]
fn update_frontier_store() {
    let ctx = LedgerWithReceiveBlock::new();
    assert_eq!(
        ctx.ledger()
            .store
            .frontier()
            .get(ctx.txn.txn(), &ctx.open_block.hash()),
        Account::zero()
    );
    assert_eq!(
        ctx.ledger()
            .store
            .frontier()
            .get(ctx.txn.txn(), &ctx.receive_block.hash()),
        ctx.receiver_account
    );
}

#[test]
fn update_balance() {
    let ctx = LedgerWithReceiveBlock::new();
    assert_eq!(
        ctx.ledger()
            .account_balance(ctx.txn.txn(), &DEV_GENESIS_ACCOUNT, false),
        Amount::new(25)
    );
    assert_eq!(
        ctx.ledger()
            .account_balance(ctx.txn.txn(), &ctx.receiver_account, false),
        ctx.expected_receiver_balance
    );
}

#[test]
fn update_vote_weight() {
    let ctx = LedgerWithReceiveBlock::new();
    assert_eq!(
        ctx.ledger().weight(&ctx.receiver_account),
        ctx.expected_receiver_balance
    );
    assert_eq!(ctx.ledger().weight(&DEV_GENESIS_ACCOUNT), Amount::new(25));
}

#[test]
fn update_account_receivable() {
    let ctx = LedgerWithReceiveBlock::new();
    assert_eq!(
        ctx.ledger()
            .account_receivable(ctx.txn.txn(), &ctx.receiver_account, false),
        Amount::zero()
    );
}

#[test]
fn update_latest_block() {
    let ctx = LedgerWithReceiveBlock::new();
    assert_eq!(
        ctx.ledger().latest(ctx.txn.txn(), &ctx.receiver_account),
        Some(ctx.receive_block.hash())
    );
}

#[test]
fn receive_fork() {
    let mut ctx = LedgerWithOpenBlock::new();

    let send = ctx.ledger_context.process_send_from_genesis(
        ctx.txn.as_mut(),
        &ctx.receiver_account,
        Amount::new(1),
    );

    ctx.ledger_context
        .process_change(ctx.txn.as_mut(), &ctx.receiver_key, Account::from(1000));

    let mut receive_fork = BlockBuilder::receive()
        .previous(ctx.open_block.hash())
        .source(send.hash())
        .sign(&ctx.receiver_key)
        .without_sideband()
        .build()
        .unwrap();

    let result = ctx.ledger_context.ledger.process(
        ctx.txn.as_mut(),
        &mut receive_fork,
        SignatureVerification::Unknown,
    );

    assert_eq!(result.code, ProcessResult::Fork);
}

#[test]
fn fail_double_receive() {
    let mut ctx = LedgerWithOpenBlock::new();

    let mut double_receive = BlockBuilder::receive()
        .previous(ctx.open_block.hash())
        .source(ctx.send_block.hash())
        .sign(&ctx.receiver_key)
        .build()
        .unwrap();

    let result = ctx.ledger_context.ledger.process(
        ctx.txn.as_mut(),
        &mut double_receive,
        SignatureVerification::Unknown,
    );

    assert_eq!(result.code, ProcessResult::Unreceivable);
}

#[test]
fn fail_old() {
    let mut ctx = LedgerWithReceiveBlock::new();

    let result = ctx.ledger_context.ledger.process(
        ctx.txn.as_mut(),
        &mut ctx.receive_block,
        SignatureVerification::Unknown,
    );

    assert_eq!(result.code, ProcessResult::Old);
}

#[test]
fn fail_gap_source() {
    let mut ctx = LedgerWithOpenBlock::new();

    let mut receive = BlockBuilder::receive()
        .previous(ctx.open_block.hash())
        .source(BlockHash::from(1))
        .sign(&ctx.receiver_key)
        .build()
        .unwrap();

    let result = ctx.ledger_context.ledger.process(
        ctx.txn.as_mut(),
        &mut receive,
        SignatureVerification::Unknown,
    );

    assert_eq!(result.code, ProcessResult::GapSource);
}

#[test]
fn fail_bad_signature() {
    let mut ctx = LedgerWithOpenBlock::new();

    let send = ctx.ledger_context.process_send_from_genesis(
        ctx.txn.as_mut(),
        &ctx.receiver_account,
        Amount::new(1),
    );

    let mut receive = BlockBuilder::receive()
        .previous(ctx.open_block.hash())
        .source(send.hash())
        .sign(&KeyPair::new())
        .build()
        .unwrap();

    let result = ctx.ledger_context.ledger.process(
        ctx.txn.as_mut(),
        &mut receive,
        SignatureVerification::Unknown,
    );

    assert_eq!(result.code, ProcessResult::BadSignature);
}

#[test]
fn fail_gap_previous_unopened() {
    let mut ctx = LedgerWithSendBlock::new();

    let mut receive = BlockBuilder::receive()
        .previous(BlockHash::from(1))
        .source(ctx.send_block.hash())
        .sign(&ctx.receiver_key)
        .build()
        .unwrap();

    let result = ctx.ledger_context.ledger.process(
        ctx.txn.as_mut(),
        &mut receive,
        SignatureVerification::Unknown,
    );

    assert_eq!(result.code, ProcessResult::GapPrevious);
}

#[test]
fn fail_gap_previous_opened() {
    let mut ctx = LedgerWithOpenBlock::new();

    let send2 = ctx.ledger_context.process_send_from_genesis(
        ctx.txn.as_mut(),
        &ctx.receiver_account,
        Amount::new(1),
    );

    let mut receive = BlockBuilder::receive()
        .previous(BlockHash::from(1))
        .source(send2.hash())
        .sign(&ctx.receiver_key)
        .build()
        .unwrap();

    let result = ctx.ledger_context.ledger.process(
        ctx.txn.as_mut(),
        &mut receive,
        SignatureVerification::Unknown,
    );

    assert_eq!(result.code, ProcessResult::GapPrevious);
}

#[test]
fn fail_fork_previous() {
    let mut ctx = LedgerWithOpenBlock::new();

    let receivable = ctx.ledger_context.process_send_from_genesis(
        ctx.txn.as_mut(),
        &ctx.receiver_account,
        Amount::new(1),
    );

    let mut fork_send = BlockBuilder::send()
        .previous(ctx.open_block.hash())
        .destination(Account::from(1))
        .balance(Amount::zero())
        .sign(ctx.receiver_key.clone())
        .without_sideband()
        .build()
        .unwrap();

    assert_eq!(
        ctx.ledger_context
            .ledger
            .process(
                ctx.txn.as_mut(),
                &mut fork_send,
                SignatureVerification::Unknown
            )
            .code,
        ProcessResult::Progress
    );

    let mut fork_receive = BlockBuilder::receive()
        .previous(ctx.open_block.hash())
        .source(receivable.hash())
        .sign(&ctx.receiver_key)
        .build()
        .unwrap();

    let result = ctx.ledger_context.ledger.process(
        ctx.txn.as_mut(),
        &mut fork_receive,
        SignatureVerification::Unknown,
    );

    assert_eq!(result.code, ProcessResult::Fork);
}

#[test]
fn fail_receive_received_source() {
    let mut ctx = LedgerWithOpenBlock::new();

    let receivable1 = ctx.ledger_context.process_send_from_genesis(
        ctx.txn.as_mut(),
        &ctx.receiver_account,
        Amount::new(1),
    );

    let receivable2 = ctx.ledger_context.process_send_from_genesis(
        ctx.txn.as_mut(),
        &ctx.receiver_account,
        Amount::new(1),
    );

    ctx.ledger_context
        .process_receive(ctx.txn.as_mut(), &receivable1, &ctx.receiver_key);

    let mut fork_receive = BlockBuilder::receive()
        .previous(ctx.open_block.hash())
        .source(receivable2.hash())
        .sign(&ctx.receiver_key)
        .build()
        .unwrap();

    let result = ctx.ledger_context.ledger.process(
        ctx.txn.as_mut(),
        &mut fork_receive,
        SignatureVerification::Unknown,
    );

    assert_eq!(result.code, ProcessResult::Fork);
}

// Make sure old block types can't be inserted after a state block.
#[test]
fn receive_after_state_fail() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    let send = ctx.process_state_send(
        txn.as_mut(),
        &DEV_GENESIS_KEY,
        *DEV_GENESIS_ACCOUNT,
        Amount::new(1),
    );

    let mut receive = BlockBuilder::receive()
        .previous(send.hash())
        .source(send.hash())
        .sign(&DEV_GENESIS_KEY.clone())
        .build()
        .unwrap();

    let result = ctx
        .ledger
        .process(txn.as_mut(), &mut receive, SignatureVerification::Unknown);

    assert_eq!(result.code, ProcessResult::BlockPosition);
}

#[test]
fn receive_from_state_block() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    let destination = KeyPair::new();
    let destination_account = destination.public_key().into();

    let send1 = ctx.process_state_send(
        txn.as_mut(),
        &DEV_GENESIS_KEY,
        destination_account,
        Amount::new(50),
    );

    let send2 = ctx.process_state_send(
        txn.as_mut(),
        &DEV_GENESIS_KEY,
        destination_account,
        Amount::new(50),
    );

    ctx.process_open(txn.as_mut(), &send1, &destination);
    let receive = ctx.process_state_receive(txn.as_mut(), &send2, &destination);

    assert_eq!(
        ctx.ledger.balance(txn.txn(), &receive.hash()),
        Amount::new(100)
    );
    assert_eq!(
        ctx.ledger.weight(&DEV_GENESIS_ACCOUNT),
        DEV_CONSTANTS.genesis_amount
    )
}
