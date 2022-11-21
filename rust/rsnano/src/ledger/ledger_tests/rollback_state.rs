use crate::{
    core::{Account, Amount, Block, BlockBuilder, Epoch, KeyPair, PendingInfo, PendingKey},
    ledger::DEV_GENESIS_KEY,
    DEV_CONSTANTS, DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH,
};

use super::LedgerContext;

#[test]
fn rollback_send() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    let send = ctx.process_state_send(
        txn.as_mut(),
        &DEV_GENESIS_KEY,
        *DEV_GENESIS_ACCOUNT,
        Amount::new(50),
    );

    ctx.ledger
        .rollback(txn.as_mut(), &send.hash(), &mut Vec::new())
        .unwrap();

    assert_eq!(
        ctx.ledger.store.block().exists(txn.txn(), &send.hash()),
        false
    );
    assert_eq!(
        ctx.ledger
            .account_balance(txn.txn(), &DEV_GENESIS_ACCOUNT, false),
        DEV_CONSTANTS.genesis_amount
    );
    assert_eq!(
        ctx.ledger.weight(&DEV_GENESIS_ACCOUNT),
        DEV_CONSTANTS.genesis_amount
    );
    assert_eq!(
        ctx.ledger.store.pending().get(
            txn.txn(),
            &PendingKey::new(*DEV_GENESIS_ACCOUNT, send.hash())
        ),
        None
    );
    assert_eq!(
        ctx.ledger
            .store
            .block()
            .successor(txn.txn(), &DEV_GENESIS_HASH),
        None
    );
}

#[test]
fn rollback_receive() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    let send = ctx.process_state_send(
        txn.as_mut(),
        &DEV_GENESIS_KEY,
        *DEV_GENESIS_ACCOUNT,
        Amount::new(50),
    );

    let receive = ctx.process_state_receive(txn.as_mut(), &send, &DEV_GENESIS_KEY);

    ctx.ledger
        .rollback(txn.as_mut(), &receive.hash(), &mut Vec::new())
        .unwrap();

    assert_eq!(
        ctx.ledger.store.block().exists(txn.txn(), &receive.hash()),
        false
    );
    assert_eq!(
        ctx.ledger
            .account_balance(txn.txn(), &DEV_GENESIS_ACCOUNT, false),
        DEV_CONSTANTS.genesis_amount - Amount::new(50)
    );
    assert_eq!(
        ctx.ledger.weight(&DEV_GENESIS_ACCOUNT),
        DEV_CONSTANTS.genesis_amount - Amount::new(50)
    );
    assert_eq!(
        ctx.ledger.store.pending().get(
            txn.txn(),
            &PendingKey::new(*DEV_GENESIS_ACCOUNT, send.hash())
        ),
        Some(PendingInfo {
            source: *DEV_GENESIS_ACCOUNT,
            amount: Amount::new(50),
            epoch: Epoch::Epoch0
        })
    );
    assert_eq!(
        ctx.ledger.store.block().successor(txn.txn(), &send.hash()),
        None
    );
}

#[test]
fn rollback_received_send() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    let destination = KeyPair::new();
    let destination_account = destination.public_key().into();
    let send = ctx.process_state_send(
        txn.as_mut(),
        &DEV_GENESIS_KEY,
        destination_account,
        Amount::new(50),
    );

    let open = ctx.process_state_open(txn.as_mut(), &send, &destination);

    ctx.ledger
        .rollback(txn.as_mut(), &send.hash(), &mut Vec::new())
        .unwrap();

    assert_eq!(
        ctx.ledger.store.pending().exists(
            txn.txn(),
            &PendingKey::new(*DEV_GENESIS_ACCOUNT, send.hash())
        ),
        false
    );
    assert_eq!(
        ctx.ledger.store.block().exists(txn.txn(), &send.hash()),
        false
    );
    assert_eq!(
        ctx.ledger.store.block().exists(txn.txn(), &open.hash()),
        false
    );
    assert_eq!(
        ctx.ledger
            .account_balance(txn.txn(), &DEV_GENESIS_ACCOUNT, false),
        DEV_CONSTANTS.genesis_amount
    );
    assert_eq!(
        ctx.ledger.weight(&DEV_GENESIS_ACCOUNT),
        DEV_CONSTANTS.genesis_amount
    );
    assert_eq!(
        ctx.ledger
            .account_balance(txn.txn(), &destination_account, false),
        Amount::zero()
    );
    assert_eq!(ctx.ledger.store.account().count(txn.txn()), 1);
}

#[test]
fn rollback_rep_change() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    let representative = Account::from(1);

    let change = ctx.process_state_change(txn.as_mut(), &DEV_GENESIS_KEY, representative);

    ctx.ledger
        .rollback(txn.as_mut(), &change.hash(), &mut Vec::new())
        .unwrap();

    assert_eq!(
        ctx.ledger.store.block().exists(txn.txn(), &change.hash()),
        false
    );
    assert_eq!(
        ctx.ledger
            .account_balance(txn.txn(), &DEV_GENESIS_ACCOUNT, false),
        DEV_CONSTANTS.genesis_amount
    );
    assert_eq!(
        ctx.ledger.weight(&DEV_GENESIS_ACCOUNT),
        DEV_CONSTANTS.genesis_amount
    );
    assert_eq!(ctx.ledger.weight(&representative), Amount::zero());
}

#[test]
fn rollback_open() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    let destination = KeyPair::new();
    let destination_account = destination.public_key().into();
    let send = ctx.process_state_send(
        txn.as_mut(),
        &DEV_GENESIS_KEY,
        destination_account,
        Amount::new(50),
    );

    let open = ctx.process_state_open(txn.as_mut(), &send, &destination);

    ctx.ledger
        .rollback(txn.as_mut(), &open.hash(), &mut Vec::new())
        .unwrap();

    assert_eq!(
        ctx.ledger.store.block().exists(txn.txn(), &open.hash()),
        false
    );
    assert_eq!(
        ctx.ledger
            .account_balance(txn.txn(), &destination_account, false),
        Amount::zero()
    );
    assert_eq!(
        ctx.ledger.weight(&DEV_GENESIS_ACCOUNT),
        DEV_CONSTANTS.genesis_amount - Amount::new(50)
    );
    assert_eq!(
        ctx.ledger
            .store
            .pending()
            .get(
                txn.txn(),
                &PendingKey::new(destination_account, send.hash())
            )
            .unwrap(),
        PendingInfo {
            source: *DEV_GENESIS_ACCOUNT,
            amount: Amount::new(50),
            epoch: Epoch::Epoch0
        }
    );
    assert_eq!(ctx.ledger.store.account().count(txn.txn()), 1);
}

#[test]
fn rollback_send_with_rep_change() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    let representative = Account::from(1);
    let mut send = BlockBuilder::state()
        .account(*DEV_GENESIS_ACCOUNT)
        .previous(*DEV_GENESIS_HASH)
        .representative(representative)
        .balance(DEV_CONSTANTS.genesis_amount - Amount::new(50))
        .link(*DEV_GENESIS_ACCOUNT)
        .sign(&DEV_GENESIS_KEY)
        .build()
        .unwrap();

    ctx.process(txn.as_mut(), &mut send);

    ctx.ledger
        .rollback(txn.as_mut(), &send.hash(), &mut Vec::new())
        .unwrap();

    assert_eq!(
        ctx.ledger.store.block().exists(txn.txn(), &send.hash()),
        false
    );
    assert_eq!(
        ctx.ledger
            .account_balance(txn.txn(), &DEV_GENESIS_ACCOUNT, false),
        DEV_CONSTANTS.genesis_amount
    );
    assert_eq!(
        ctx.ledger.weight(&DEV_GENESIS_ACCOUNT),
        DEV_CONSTANTS.genesis_amount
    );
    assert_eq!(ctx.ledger.weight(&representative), Amount::zero());
}

#[test]
fn rollback_receive_with_rep_change() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    let representative = Account::from(1);
    let send = ctx.process_state_send(
        txn.as_mut(),
        &DEV_GENESIS_KEY,
        *DEV_GENESIS_ACCOUNT,
        Amount::new(50),
    );

    let mut receive = BlockBuilder::state()
        .account(*DEV_GENESIS_ACCOUNT)
        .previous(send.hash())
        .representative(representative)
        .balance(DEV_CONSTANTS.genesis_amount)
        .link(send.hash())
        .sign(&DEV_GENESIS_KEY)
        .build()
        .unwrap();

    ctx.process(txn.as_mut(), &mut receive);

    ctx.ledger
        .rollback(txn.as_mut(), &receive.hash(), &mut Vec::new())
        .unwrap();

    assert_eq!(
        ctx.ledger.store.block().exists(txn.txn(), &receive.hash()),
        false
    );
    assert_eq!(
        ctx.ledger
            .account_balance(txn.txn(), &DEV_GENESIS_ACCOUNT, false),
        DEV_CONSTANTS.genesis_amount - Amount::new(50)
    );
    assert_eq!(
        ctx.ledger.weight(&DEV_GENESIS_ACCOUNT),
        DEV_CONSTANTS.genesis_amount - Amount::new(50)
    );
    assert_eq!(ctx.ledger.weight(&representative), Amount::zero());
}
