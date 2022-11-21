use crate::{
    core::{
        Account, Amount, Block, BlockBuilder, BlockEnum, BlockHash, KeyPair, SignatureVerification,
    },
    ledger::{
        ledger_tests::{LedgerContext, LedgerWithOpenBlock, LedgerWithSendBlock},
        ProcessResult, DEV_GENESIS_KEY,
    },
    DEV_CONSTANTS, DEV_GENESIS_ACCOUNT,
};

#[test]
fn update_sideband() {
    let ctx = LedgerWithOpenBlock::new();
    let sideband = ctx.open_block.sideband().unwrap();
    assert_eq!(sideband.account, ctx.receiver_account);
    assert_eq!(sideband.balance, ctx.amount_sent);
    assert_eq!(sideband.height, 1);
}

#[test]
fn save_block() {
    let ctx = LedgerWithOpenBlock::new();

    let loaded_open = ctx
        .ledger()
        .store
        .block()
        .get(ctx.txn.txn(), &ctx.open_block.hash())
        .unwrap();

    let BlockEnum::Open(loaded_open) = loaded_open else{panic!("not an open block")};
    assert_eq!(loaded_open, ctx.open_block);
    assert_eq!(
        loaded_open.sideband().unwrap(),
        ctx.open_block.sideband().unwrap()
    );
}

#[test]
fn update_block_amount() {
    let ctx = LedgerWithOpenBlock::new();
    assert_eq!(
        ctx.ledger().amount(ctx.txn.txn(), &ctx.open_block.hash()),
        Some(ctx.amount_sent)
    );
    assert_eq!(
        ctx.ledger()
            .store
            .block()
            .account_calculated(&ctx.open_block),
        ctx.receiver_account
    );
}

#[test]
fn update_frontier_store() {
    let ctx = LedgerWithOpenBlock::new();
    assert_eq!(
        ctx.ledger()
            .store
            .frontier()
            .get(ctx.txn.txn(), &ctx.open_block.hash()),
        ctx.receiver_account
    );
}

#[test]
fn update_account_balance() {
    let ctx = LedgerWithOpenBlock::new();
    assert_eq!(
        ctx.ledger()
            .account_balance(ctx.txn.txn(), &ctx.receiver_account, false),
        ctx.amount_sent
    );
}

#[test]
fn update_account_receivable() {
    let ctx = LedgerWithOpenBlock::new();
    assert_eq!(
        ctx.ledger()
            .account_receivable(ctx.txn.txn(), &ctx.receiver_account, false),
        Amount::zero()
    );
}

#[test]
fn update_vote_weight() {
    let ctx = LedgerWithOpenBlock::new();
    assert_eq!(
        ctx.ledger().weight(&DEV_GENESIS_ACCOUNT),
        DEV_CONSTANTS.genesis_amount - ctx.amount_sent
    );
    assert_eq!(ctx.ledger().weight(&ctx.receiver_account), ctx.amount_sent);
}

#[test]
fn update_sender_account_info() {
    let ctx = LedgerWithOpenBlock::new();
    let sender_info = ctx
        .ledger()
        .store
        .account()
        .get(ctx.txn.txn(), &DEV_GENESIS_ACCOUNT)
        .unwrap();
    assert_eq!(sender_info.head, ctx.send_block.hash());
}

#[test]
fn update_receiver_account_info() {
    let ctx = LedgerWithOpenBlock::new();
    let receiver_info = ctx
        .ledger()
        .store
        .account()
        .get(ctx.txn.txn(), &ctx.receiver_account)
        .unwrap();
    assert_eq!(receiver_info.head, ctx.open_block.hash());
}

#[test]
fn fail_fork() {
    let mut ctx = LedgerWithOpenBlock::new();
    let mut open_fork = BlockBuilder::open()
        .source(ctx.send_block.hash())
        .representative(Account::from(1000))
        .account(ctx.receiver_account)
        .sign(&ctx.receiver_key)
        .build()
        .unwrap();

    let result = ctx.ledger_context.ledger.process(
        ctx.txn.as_mut(),
        &mut open_fork,
        SignatureVerification::Unknown,
    );

    assert_eq!(result.code, ProcessResult::Fork);
}

#[test]
fn fail_fork_previous() {
    let mut ctx = LedgerWithOpenBlock::new();

    let send2 = ctx.ledger_context.process_send_from_genesis(
        ctx.txn.as_mut(),
        &ctx.receiver_account,
        Amount::new(1),
    );

    let mut open_fork = BlockBuilder::open()
        .source(send2.hash())
        .account(ctx.receiver_account)
        .sign(&ctx.receiver_key)
        .build()
        .unwrap();

    let result = ctx.ledger_context.ledger.process(
        ctx.txn.as_mut(),
        &mut open_fork,
        SignatureVerification::Unknown,
    );

    assert_eq!(result.code, ProcessResult::Fork);
}

#[test]
fn process_duplicate_open_fails() {
    let mut ctx = LedgerWithOpenBlock::new();

    let result = ctx.ledger_context.ledger.process(
        ctx.txn.as_mut(),
        &mut ctx.open_block,
        SignatureVerification::Unknown,
    );

    assert_eq!(result.code, ProcessResult::Old);
}

#[test]
fn fail_gap_source() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();
    let keypair = KeyPair::new();

    let mut open = BlockBuilder::open()
        .source(BlockHash::from(1))
        .account(keypair.public_key().into())
        .sign(&keypair)
        .build()
        .unwrap();

    let result = ctx
        .ledger
        .process(txn.as_mut(), &mut open, SignatureVerification::Unknown);

    assert_eq!(result.code, ProcessResult::GapSource);
}

#[test]
fn fail_bad_signature() {
    let mut ctx = LedgerWithSendBlock::new();
    let bad_keys = KeyPair::new();

    let mut open = BlockBuilder::open()
        .source(ctx.send_block.hash())
        .account(ctx.receiver_account)
        .sign(&bad_keys)
        .build()
        .unwrap();

    let result = ctx.ledger_context.ledger.process(
        ctx.txn.as_mut(),
        &mut open,
        SignatureVerification::Unknown,
    );
    assert_eq!(result.code, ProcessResult::BadSignature);
}

#[test]
fn fail_account_mismatch() {
    let mut ctx = LedgerWithSendBlock::new();
    let bad_key = KeyPair::new();

    let mut open = BlockBuilder::open()
        .source(ctx.send_block.hash())
        .account(bad_key.public_key().into())
        .sign(&bad_key)
        .build()
        .unwrap();

    let result = ctx.ledger_context.ledger.process(
        ctx.txn.as_mut(),
        &mut open,
        SignatureVerification::Unknown,
    );

    assert_eq!(result.code, ProcessResult::Unreceivable);
}

#[test]
fn state_open_fork() {
    let mut ctx = LedgerWithSendBlock::new();

    ctx.ledger_context
        .process_state_open(ctx.txn.as_mut(), &ctx.send_block, &ctx.receiver_key);

    let mut open2 = BlockBuilder::open()
        .source(ctx.send_block.hash())
        .account(ctx.receiver_account)
        .sign(&ctx.receiver_key)
        .build()
        .unwrap();

    let result = ctx.ledger_context.ledger.process(
        ctx.txn.as_mut(),
        &mut open2,
        SignatureVerification::Unknown,
    );

    assert_eq!(result.code, ProcessResult::Fork);
}

#[test]
fn open_from_state_block() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();
    let destination = KeyPair::new();
    let destination_account = destination.public_key().into();
    let amount_sent = Amount::new(50);
    let send = ctx.process_state_send(
        txn.as_mut(),
        &DEV_GENESIS_KEY,
        destination_account,
        amount_sent,
    );

    let mut open = BlockBuilder::open()
        .source(send.hash())
        .account(destination_account)
        .representative(*DEV_GENESIS_ACCOUNT)
        .sign(&destination)
        .build()
        .unwrap();

    ctx.process(txn.as_mut(), &mut open);

    assert_eq!(ctx.ledger.balance(txn.txn(), &open.hash()), amount_sent);
    assert_eq!(
        ctx.ledger.weight(&DEV_GENESIS_ACCOUNT),
        DEV_CONSTANTS.genesis_amount
    );
}
