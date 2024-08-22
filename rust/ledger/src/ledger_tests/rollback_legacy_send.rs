use rsnano_core::PendingKey;
use rsnano_store_lmdb::LmdbWriteTransaction;

use crate::{
    ledger_constants::{DEV_GENESIS_PUB_KEY, LEDGER_CONSTANTS_STUB},
    ledger_tests::setup_legacy_open_block,
    DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH,
};

use super::{
    helpers::{setup_legacy_send_block, LegacySendBlockResult},
    LedgerContext,
};

#[test]
fn update_vote_weight() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    rollback_send_block(&ctx, &mut txn);

    assert_eq!(
        ctx.ledger.weight(&DEV_GENESIS_PUB_KEY),
        LEDGER_CONSTANTS_STUB.genesis_amount
    );
}

#[test]
fn update_account_store() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    rollback_send_block(&ctx, &mut txn);

    let account_info = ctx.ledger.account_info(&txn, &DEV_GENESIS_ACCOUNT).unwrap();
    assert_eq!(account_info.block_count, 1);
    assert_eq!(account_info.head, *DEV_GENESIS_HASH);
    assert_eq!(account_info.balance, LEDGER_CONSTANTS_STUB.genesis_amount);
    assert_eq!(ctx.ledger.account_count(), 1);
}

#[test]
fn remove_from_pending_store() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    let send = rollback_send_block(&ctx, &mut txn);

    let pending = ctx.ledger.any().get_pending(
        &txn,
        &PendingKey::new(send.destination.account(), send.send_block.hash()),
    );
    assert_eq!(pending, None);
}

#[test]
fn update_confirmation_height_store() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    rollback_send_block(&ctx, &mut txn);

    let conf_height = ctx
        .ledger
        .get_confirmation_height(&txn, &DEV_GENESIS_ACCOUNT)
        .unwrap();

    assert_eq!(conf_height.frontier, *DEV_GENESIS_HASH);
    assert_eq!(conf_height.height, 1);
}

#[test]
fn rollback_dependent_blocks_too() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    let open = setup_legacy_open_block(&ctx, &mut txn);

    // Rollback send block. This requires the rollback of the open block first.
    ctx.ledger
        .rollback(&mut txn, &open.send_block.hash())
        .unwrap();

    assert_eq!(
        ctx.ledger.any().account_balance(&txn, &DEV_GENESIS_ACCOUNT),
        Some(LEDGER_CONSTANTS_STUB.genesis_amount)
    );

    assert_eq!(
        ctx.ledger
            .any()
            .account_balance(&txn, &open.destination.account()),
        None
    );

    assert!(ctx
        .ledger
        .account_info(&txn, &open.destination.account())
        .is_none());

    let pending = ctx.ledger.any().get_pending(
        &txn,
        &PendingKey::new(open.destination.account(), *DEV_GENESIS_HASH),
    );
    assert_eq!(pending, None);
}

fn rollback_send_block<'a>(
    ctx: &'a LedgerContext,
    txn: &mut LmdbWriteTransaction,
) -> LegacySendBlockResult<'a> {
    let send = setup_legacy_send_block(ctx, txn);
    ctx.ledger.rollback(txn, &send.send_block.hash()).unwrap();
    send
}
