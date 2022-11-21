use crate::{
    core::{
        Account, Amount, Block, BlockBuilder, BlockEnum, BlockHash, KeyPair, SignatureVerification,
    },
    ledger::{
        ledger_tests::{LedgerContext, LedgerWithSendBlock},
        ProcessResult, DEV_GENESIS_KEY,
    },
    DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH,
};

#[test]
fn save_block() {
    let ctx = LedgerWithSendBlock::new();

    let loaded_send = ctx
        .ledger()
        .store
        .block()
        .get(ctx.txn.txn(), &ctx.send_block.hash())
        .unwrap();

    let BlockEnum::Send(loaded_send) = loaded_send else {panic!("not a send block")};
    assert_eq!(loaded_send, ctx.send_block);
    assert_eq!(
        loaded_send.sideband().unwrap(),
        ctx.send_block.sideband().unwrap()
    );
}

#[test]
fn update_sideband() {
    let ctx = LedgerWithSendBlock::new();
    let sideband = ctx.send_block.sideband().unwrap();
    assert_eq!(sideband.account, *DEV_GENESIS_ACCOUNT);
    assert_eq!(sideband.height, 2);
    assert_eq!(sideband.balance, Amount::new(50));
}

#[test]
fn update_block_amount() {
    let ctx = LedgerWithSendBlock::new();
    assert_eq!(
        ctx.ledger().amount(ctx.txn.txn(), &ctx.send_block.hash()),
        Some(ctx.amount_sent)
    );
}

#[test]
fn update_receivable() {
    let ctx = LedgerWithSendBlock::new();
    assert_eq!(
        ctx.ledger()
            .account_receivable(ctx.txn.txn(), &ctx.receiver_account, false),
        ctx.amount_sent
    );
}

#[test]
fn update_frontier_store() {
    let ctx = LedgerWithSendBlock::new();
    assert_eq!(
        ctx.ledger()
            .store
            .frontier()
            .get(ctx.txn.txn(), &DEV_GENESIS_HASH),
        Account::zero()
    );
    assert_eq!(
        ctx.ledger()
            .store
            .frontier()
            .get(ctx.txn.txn(), &ctx.send_block.hash()),
        *DEV_GENESIS_ACCOUNT
    );
}

#[test]
fn update_account_info() {
    let ctx = LedgerWithSendBlock::new();
    let account_info = ctx
        .ledger()
        .store
        .account()
        .get(ctx.txn.txn(), &DEV_GENESIS_ACCOUNT)
        .unwrap();
    assert_eq!(account_info.block_count, 2);
    assert_eq!(account_info.head, ctx.send_block.hash());
}

#[test]
fn update_vote_weight() {
    let ctx = LedgerWithSendBlock::new();
    assert_eq!(ctx.ledger().weight(&ctx.receiver_account), Amount::zero());
    assert_eq!(ctx.ledger().weight(&DEV_GENESIS_ACCOUNT), Amount::new(50));
}

#[test]
fn fail_duplicate_send() {
    let mut ctx = LedgerWithSendBlock::new();

    let result = ctx.ledger_context.ledger.process(
        ctx.txn.as_mut(),
        &mut ctx.send_block,
        SignatureVerification::Unknown,
    );

    assert_eq!(result.code, ProcessResult::Old);
}

#[test]
fn fail_fork() {
    let mut ctx = LedgerWithSendBlock::new();

    let mut fork = BlockBuilder::send()
        .previous(*DEV_GENESIS_HASH)
        .destination(Account::from(1000))
        .sign(ctx.receiver_key)
        .build()
        .unwrap();

    let result = ctx.ledger_context.ledger.process(
        ctx.txn.as_mut(),
        &mut fork,
        SignatureVerification::Unknown,
    );

    assert_eq!(result.code, ProcessResult::Fork);
}

#[test]
fn fail_gap_previous() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    let mut block = BlockBuilder::send()
        .previous(BlockHash::from(1))
        .destination(Account::from(2))
        .sign(DEV_GENESIS_KEY.clone())
        .build()
        .unwrap();

    let result = ctx
        .ledger
        .process(txn.as_mut(), &mut block, SignatureVerification::Unknown);

    assert_eq!(result.code, ProcessResult::GapPrevious);
}

#[test]
fn fail_bad_signature() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    let wrong_keys = KeyPair::new();
    let mut block = BlockBuilder::send()
        .previous(*DEV_GENESIS_HASH)
        .destination(Account::from(2))
        .sign(wrong_keys)
        .build()
        .unwrap();

    let result = ctx
        .ledger
        .process(txn.as_mut(), &mut block, SignatureVerification::Unknown);

    assert_eq!(result.code, ProcessResult::BadSignature);
}

#[test]
fn fail_negative_spend() {
    let mut ctx = LedgerWithSendBlock::new();

    let mut block = BlockBuilder::send()
        .previous(ctx.send_block.hash())
        .destination(Account::from(2))
        .balance(ctx.send_block.balance() + Amount::new(1))
        .sign(DEV_GENESIS_KEY.clone())
        .build()
        .unwrap();

    let result = ctx.ledger_context.ledger.process(
        ctx.txn.as_mut(),
        &mut block,
        SignatureVerification::Unknown,
    );
    assert_eq!(result.code, ProcessResult::NegativeSpend);
}

// Make sure old block types can't be inserted after a state block.
#[test]
fn send_after_state_fail() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    let send1 = ctx.process_state_send(
        txn.as_mut(),
        &DEV_GENESIS_KEY,
        *DEV_GENESIS_ACCOUNT,
        Amount::new(1),
    );

    let mut send2 = BlockBuilder::send()
        .previous(send1.hash())
        .destination(*DEV_GENESIS_ACCOUNT)
        .balance(Amount::zero())
        .sign(DEV_GENESIS_KEY.clone())
        .build()
        .unwrap();

    let result = ctx
        .ledger
        .process(txn.as_mut(), &mut send2, SignatureVerification::Unknown);

    assert_eq!(result.code, ProcessResult::BlockPosition);
}
