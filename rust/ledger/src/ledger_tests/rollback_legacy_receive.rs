use crate::{
    ledger_tests::{helpers::setup_legacy_receive_block, LedgerContext},
    DEV_GENESIS_ACCOUNT,
};
use rsnano_core::PendingKey;

#[test]
fn clear_successor() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();
    let receive = setup_legacy_receive_block(&ctx, &mut txn);

    ctx.ledger
        .rollback(&mut txn, &receive.receive_block.hash())
        .unwrap();

    assert_eq!(
        ctx.ledger
            .store
            .block
            .successor(&txn, &receive.open_block.hash()),
        None
    );
}

#[test]
fn update_account_info() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();
    let receive = setup_legacy_receive_block(&ctx, &mut txn);

    ctx.ledger
        .rollback(&mut txn, &receive.receive_block.hash())
        .unwrap();

    let account_info = ctx
        .ledger
        .account_info(&txn, &receive.destination.account())
        .unwrap();

    assert_eq!(account_info.head, receive.open_block.hash());
    assert_eq!(account_info.block_count, 1);
    assert_eq!(
        account_info.balance,
        receive.open_block.sideband().unwrap().balance
    );
}

#[test]
fn rollback_pending_info() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();
    let receive = setup_legacy_receive_block(&ctx, &mut txn);

    ctx.ledger
        .rollback(&mut txn, &receive.receive_block.hash())
        .unwrap();

    let pending = ctx
        .ledger
        .any()
        .get_pending(
            &txn,
            &PendingKey::new(receive.destination.account(), receive.send_block.hash()),
        )
        .unwrap();

    assert_eq!(pending.source, *DEV_GENESIS_ACCOUNT);
    assert_eq!(pending.amount, receive.amount_received);
}

#[test]
fn rollback_vote_weight() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();
    let receive = setup_legacy_receive_block(&ctx, &mut txn);

    ctx.ledger
        .rollback(&mut txn, &receive.receive_block.hash())
        .unwrap();

    assert_eq!(
        ctx.ledger.weight(&receive.destination.public_key()),
        receive.expected_balance - receive.amount_received
    );
}
