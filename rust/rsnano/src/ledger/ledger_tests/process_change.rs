use crate::{
    core::{
        Account, Amount, Block, BlockBuilder, BlockEnum, BlockHash, KeyPair, SignatureVerification,
    },
    ledger::{ProcessResult, DEV_GENESIS_KEY},
    DEV_CONSTANTS, DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH,
};

use super::LedgerContext;

#[test]
fn update_sideband() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    let change = ctx.process_change(txn.as_mut(), &DEV_GENESIS_KEY, Account::from(1000));

    let sideband = change.sideband().unwrap();
    assert_eq!(sideband.account, *DEV_GENESIS_ACCOUNT);
    assert_eq!(sideband.balance, DEV_CONSTANTS.genesis_amount);
    assert_eq!(sideband.height, 2);
}

#[test]
fn save_block() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    let change = ctx.process_change(txn.as_mut(), &DEV_GENESIS_KEY, Account::from(1000));

    let loaded_block = ctx
        .ledger
        .store
        .block()
        .get(txn.txn(), &change.hash())
        .unwrap();

    let BlockEnum::Change(loaded_block) = loaded_block else{panic!("not a change block")};
    assert_eq!(loaded_block, change);
    assert_eq!(loaded_block.sideband().unwrap(), change.sideband().unwrap());
}

#[test]
fn update_frontier_store() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    let change = ctx.process_change(txn.as_mut(), &DEV_GENESIS_KEY, Account::from(1000));

    let account = ctx.ledger.store.frontier().get(txn.txn(), &change.hash());
    assert_eq!(account, *DEV_GENESIS_ACCOUNT);

    let account = ctx
        .ledger
        .store
        .frontier()
        .get(txn.txn(), &DEV_GENESIS_HASH);
    assert_eq!(account, Account::zero());
}

#[test]
fn update_vote_weight() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    let change = ctx.process_change(txn.as_mut(), &DEV_GENESIS_KEY, Account::from(1000));

    assert_eq!(ctx.ledger.weight(&DEV_GENESIS_ACCOUNT), Amount::zero());
    assert_eq!(
        ctx.ledger.weight(&change.representative()),
        DEV_CONSTANTS.genesis_amount
    );
}

#[test]
fn update_account_info() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    let change = ctx.process_change(txn.as_mut(), &DEV_GENESIS_KEY, Account::from(1000));

    let account_info = ctx
        .ledger
        .store
        .account()
        .get(txn.txn(), &DEV_GENESIS_ACCOUNT)
        .unwrap();

    assert_eq!(account_info.head, change.hash());
    assert_eq!(account_info.block_count, 2);
    assert_eq!(account_info.balance, DEV_CONSTANTS.genesis_amount);
    assert_eq!(account_info.representative, change.representative());
}

#[test]
fn fail_old() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    let mut change = ctx.process_change(txn.as_mut(), &DEV_GENESIS_KEY, Account::from(1000));

    let result = ctx
        .ledger
        .process(txn.as_mut(), &mut change, SignatureVerification::Unknown);
    assert_eq!(result.code, ProcessResult::Old);
}

#[test]
fn fail_gap_previous() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();
    let keypair = KeyPair::new();

    let mut block = BlockBuilder::change()
        .previous(BlockHash::from(1))
        .sign(keypair)
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

    let mut block = BlockBuilder::change()
        .previous(*DEV_GENESIS_HASH)
        .sign(wrong_keys)
        .build()
        .unwrap();

    let result = ctx
        .ledger
        .process(txn.as_mut(), &mut block, SignatureVerification::Unknown);

    assert_eq!(result.code, ProcessResult::BadSignature);
}

#[test]
fn fail_fork() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    ctx.process_change(txn.as_mut(), &DEV_GENESIS_KEY, Account::from(1000));

    let mut fork = BlockBuilder::change()
        .previous(*DEV_GENESIS_HASH)
        .representative(Account::from(12345))
        .sign(DEV_GENESIS_KEY.clone())
        .build()
        .unwrap();

    let result = ctx
        .ledger
        .process(txn.as_mut(), &mut fork, SignatureVerification::Unknown);

    assert_eq!(result.code, ProcessResult::Fork);
}

// Make sure old block types can't be inserted after a state block.
#[test]
fn change_after_state_fail() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    let send = ctx.process_state_send(
        txn.as_mut(),
        &DEV_GENESIS_KEY,
        *DEV_GENESIS_ACCOUNT,
        Amount::new(1),
    );

    let mut change = BlockBuilder::change()
        .previous(send.hash())
        .sign(DEV_GENESIS_KEY.clone())
        .build()
        .unwrap();

    let result = ctx
        .ledger
        .process(txn.as_mut(), &mut change, SignatureVerification::Unknown);

    assert_eq!(result.code, ProcessResult::BlockPosition);
}
