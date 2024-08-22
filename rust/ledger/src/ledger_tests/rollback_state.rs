use super::LedgerContext;
use crate::{
    ledger_constants::{DEV_GENESIS_PUB_KEY, LEDGER_CONSTANTS_STUB},
    ledger_tests::AccountBlockFactory,
    DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH,
};
use rsnano_core::{Amount, Epoch, PendingInfo, PendingKey, PublicKey};

#[test]
fn rollback_send() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();
    let genesis = ctx.genesis_block_factory();

    let mut send = genesis.send(&txn).build();
    ctx.ledger.process(&mut txn, &mut send).unwrap();

    ctx.ledger.rollback(&mut txn, &send.hash()).unwrap();

    assert_eq!(ctx.ledger.store.block.exists(&txn, &send.hash()), false);
    assert_eq!(
        ctx.ledger.any().account_balance(&txn, &DEV_GENESIS_ACCOUNT),
        Some(LEDGER_CONSTANTS_STUB.genesis_amount)
    );
    assert_eq!(
        ctx.ledger.weight(&DEV_GENESIS_PUB_KEY),
        LEDGER_CONSTANTS_STUB.genesis_amount
    );
    assert_eq!(
        ctx.ledger
            .any()
            .get_pending(&txn, &PendingKey::new(*DEV_GENESIS_ACCOUNT, send.hash())),
        None
    );
    assert_eq!(
        ctx.ledger.store.block.successor(&txn, &DEV_GENESIS_HASH),
        None
    );
}

#[test]
fn rollback_receive() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();
    let genesis = ctx.genesis_block_factory();

    let amount_sent = Amount::raw(50);
    let mut send = genesis
        .send(&txn)
        .amount_sent(amount_sent)
        .link(genesis.account())
        .build();
    ctx.ledger.process(&mut txn, &mut send).unwrap();

    let mut receive = genesis.receive(&txn, send.hash()).build();
    ctx.ledger.process(&mut txn, &mut receive).unwrap();

    ctx.ledger.rollback(&mut txn, &receive.hash()).unwrap();

    assert_eq!(ctx.ledger.store.block.exists(&txn, &receive.hash()), false);
    assert_eq!(
        ctx.ledger.any().account_balance(&txn, &DEV_GENESIS_ACCOUNT),
        Some(LEDGER_CONSTANTS_STUB.genesis_amount - amount_sent)
    );
    assert_eq!(
        ctx.ledger.weight(&DEV_GENESIS_PUB_KEY),
        LEDGER_CONSTANTS_STUB.genesis_amount - amount_sent
    );
    assert_eq!(
        ctx.ledger
            .any()
            .get_pending(&txn, &PendingKey::new(*DEV_GENESIS_ACCOUNT, send.hash())),
        Some(PendingInfo {
            source: *DEV_GENESIS_ACCOUNT,
            amount: amount_sent,
            epoch: Epoch::Epoch0
        })
    );
    assert_eq!(ctx.ledger.store.block.successor(&txn, &send.hash()), None);
}

#[test]
fn rollback_received_send() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();
    let genesis = ctx.genesis_block_factory();
    let destination = AccountBlockFactory::new(&ctx.ledger);

    let mut send = genesis.send(&txn).link(destination.account()).build();
    ctx.ledger.process(&mut txn, &mut send).unwrap();

    let mut open = destination.open(&txn, send.hash()).build();
    ctx.ledger.process(&mut txn, &mut open).unwrap();

    ctx.ledger.rollback(&mut txn, &send.hash()).unwrap();

    assert_eq!(
        ctx.ledger
            .any()
            .get_pending(&txn, &PendingKey::new(*DEV_GENESIS_ACCOUNT, send.hash())),
        None
    );
    assert_eq!(ctx.ledger.store.block.exists(&txn, &send.hash()), false);
    assert_eq!(ctx.ledger.store.block.exists(&txn, &open.hash()), false);
    assert_eq!(
        ctx.ledger.any().account_balance(&txn, &DEV_GENESIS_ACCOUNT),
        Some(LEDGER_CONSTANTS_STUB.genesis_amount)
    );
    assert_eq!(
        ctx.ledger.weight(&DEV_GENESIS_PUB_KEY),
        LEDGER_CONSTANTS_STUB.genesis_amount
    );
    assert_eq!(
        ctx.ledger
            .any()
            .account_balance(&txn, &destination.account()),
        None
    );
    assert_eq!(ctx.ledger.store.account.count(&txn), 1);
}

#[test]
fn rollback_rep_change() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();
    let genesis = ctx.genesis_block_factory();
    let representative = PublicKey::from(1);

    let mut change = genesis.change(&txn).representative(representative).build();
    ctx.ledger.process(&mut txn, &mut change).unwrap();

    ctx.ledger.rollback(&mut txn, &change.hash()).unwrap();

    assert_eq!(ctx.ledger.store.block.exists(&txn, &change.hash()), false);
    assert_eq!(
        ctx.ledger.any().account_balance(&txn, &DEV_GENESIS_ACCOUNT),
        Some(LEDGER_CONSTANTS_STUB.genesis_amount)
    );
    assert_eq!(
        ctx.ledger.weight(&DEV_GENESIS_PUB_KEY),
        LEDGER_CONSTANTS_STUB.genesis_amount
    );
    assert_eq!(ctx.ledger.weight(&representative), Amount::zero());
}

#[test]
fn rollback_open() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();
    let genesis = ctx.genesis_block_factory();
    let destination = AccountBlockFactory::new(&ctx.ledger);

    let amount_sent = Amount::raw(50);
    let mut send = genesis
        .send(&txn)
        .link(destination.account())
        .amount_sent(amount_sent)
        .build();
    ctx.ledger.process(&mut txn, &mut send).unwrap();

    let mut open = destination.open(&txn, send.hash()).build();
    ctx.ledger.process(&mut txn, &mut open).unwrap();

    ctx.ledger.rollback(&mut txn, &open.hash()).unwrap();

    assert_eq!(ctx.ledger.store.block.exists(&txn, &open.hash()), false);
    assert_eq!(
        ctx.ledger
            .any()
            .account_balance(&txn, &destination.account()),
        None
    );
    assert_eq!(
        ctx.ledger.weight(&DEV_GENESIS_PUB_KEY),
        LEDGER_CONSTANTS_STUB.genesis_amount - amount_sent
    );
    assert_eq!(
        ctx.ledger
            .any()
            .get_pending(&txn, &PendingKey::new(destination.account(), send.hash()))
            .unwrap(),
        PendingInfo {
            source: *DEV_GENESIS_ACCOUNT,
            amount: Amount::raw(50),
            epoch: Epoch::Epoch0
        }
    );
    assert_eq!(ctx.ledger.store.account.count(&txn), 1);
}

#[test]
fn rollback_send_with_rep_change() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();
    let genesis = ctx.genesis_block_factory();

    let representative = PublicKey::from(1);
    let mut send = genesis.send(&txn).representative(representative).build();
    ctx.ledger.process(&mut txn, &mut send).unwrap();

    ctx.ledger.rollback(&mut txn, &send.hash()).unwrap();

    assert_eq!(ctx.ledger.store.block.exists(&txn, &send.hash()), false);
    assert_eq!(
        ctx.ledger.any().account_balance(&txn, &DEV_GENESIS_ACCOUNT),
        Some(LEDGER_CONSTANTS_STUB.genesis_amount)
    );
    assert_eq!(
        ctx.ledger.weight(&DEV_GENESIS_PUB_KEY),
        LEDGER_CONSTANTS_STUB.genesis_amount
    );
    assert_eq!(ctx.ledger.weight(&representative), Amount::zero());
}

#[test]
fn rollback_receive_with_rep_change() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();
    let genesis = ctx.genesis_block_factory();

    let representative = PublicKey::from(1);
    let mut send = genesis.send(&txn).link(genesis.account()).build();
    ctx.ledger.process(&mut txn, &mut send).unwrap();

    let mut receive = genesis
        .receive(&txn, send.hash())
        .representative(representative)
        .build();
    ctx.ledger.process(&mut txn, &mut receive).unwrap();

    ctx.ledger.rollback(&mut txn, &receive.hash()).unwrap();

    assert_eq!(ctx.ledger.store.block.exists(&txn, &receive.hash()), false);
    assert_eq!(
        ctx.ledger.any().account_balance(&txn, &DEV_GENESIS_ACCOUNT),
        Some(send.balance_field().unwrap())
    );
    assert_eq!(
        ctx.ledger.weight(&DEV_GENESIS_PUB_KEY),
        send.balance_field().unwrap()
    );
    assert_eq!(ctx.ledger.weight(&representative), Amount::zero());
}
