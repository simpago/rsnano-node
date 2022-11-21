use crate::{
    core::{
        Account, Amount, Block, BlockBuilder, BlockDetails, BlockEnum, BlockHash, Epoch, KeyPair,
        Link, PendingKey, SignatureVerification, StateBlock,
    },
    ledger::{datastore::WriteTransaction, ProcessResult, DEV_GENESIS_KEY},
    DEV_CONSTANTS, DEV_GENESIS_ACCOUNT,
};

use super::LedgerContext;

#[test]
fn save_block() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    let (_, receive) = receive_50_raw_into_genesis(&ctx, txn.as_mut());

    let loaded_block = ctx
        .ledger
        .store
        .block()
        .get(txn.txn(), &receive.hash())
        .unwrap();

    let BlockEnum::State(loaded_block) = loaded_block else { panic!("not a state block")};
    assert_eq!(loaded_block, receive);
    assert_eq!(
        loaded_block.sideband().unwrap(),
        receive.sideband().unwrap()
    );
}

#[test]
fn create_sideband() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    let (_, receive) = receive_50_raw_into_genesis(&ctx, txn.as_mut());

    let sideband = receive.sideband().unwrap();
    assert_eq!(sideband.account, *DEV_GENESIS_ACCOUNT);
    assert_eq!(sideband.height, 3);
    assert_eq!(
        sideband.details,
        BlockDetails::new(Epoch::Epoch0, false, true, false)
    );
}

#[test]
fn update_vote_weight() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    let (_, receive) = receive_50_raw_into_genesis(&ctx, txn.as_mut());

    assert_eq!(ctx.ledger.weight(&DEV_GENESIS_ACCOUNT), receive.balance());
}

#[test]
fn remove_pending_info() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    let (send, _) = receive_50_raw_into_genesis(&ctx, txn.as_mut());

    assert_eq!(
        ctx.ledger.store.pending().get(
            txn.txn(),
            &PendingKey::new(*DEV_GENESIS_ACCOUNT, send.hash())
        ),
        None
    );
}

#[test]
fn receive_old_send_block() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();
    let send = ctx.process_send_from_genesis(txn.as_mut(), &DEV_GENESIS_ACCOUNT, Amount::new(50));

    let receive = ctx.process_state_receive(txn.as_mut(), &send, &DEV_GENESIS_KEY);

    let sideband = receive.sideband().unwrap();
    assert_eq!(sideband.account, *DEV_GENESIS_ACCOUNT);
    assert_eq!(sideband.height, 3);
    assert_eq!(
        sideband.details,
        BlockDetails::new(Epoch::Epoch0, false, true, false)
    );

    let loaded_block = ctx
        .ledger
        .store
        .block()
        .get(txn.txn(), &receive.hash())
        .unwrap();

    let BlockEnum::State(loaded_block) = loaded_block else { panic!("not a state block")};
    assert_eq!(loaded_block, receive);
    assert_eq!(
        loaded_block.sideband().unwrap(),
        receive.sideband().unwrap()
    );
}

#[test]
fn state_unreceivable_fail() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    let send = ctx.process_state_send(
        txn.as_mut(),
        &DEV_GENESIS_KEY,
        *DEV_GENESIS_ACCOUNT,
        Amount::new(1),
    );

    let mut receive = BlockBuilder::state()
        .account(*DEV_GENESIS_ACCOUNT)
        .previous(send.hash())
        .balance(DEV_CONSTANTS.genesis_amount)
        .link(Link::from(1))
        .sign(&DEV_GENESIS_KEY)
        .build()
        .unwrap();

    let result = ctx
        .ledger
        .process(txn.as_mut(), &mut receive, SignatureVerification::Unknown);

    assert_eq!(result.code, ProcessResult::GapSource);
}

#[test]
fn bad_amount_fail() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    let send = ctx.process_state_send(
        txn.as_mut(),
        &DEV_GENESIS_KEY,
        *DEV_GENESIS_ACCOUNT,
        Amount::new(1),
    );

    let mut receive = BlockBuilder::state()
        .account(*DEV_GENESIS_ACCOUNT)
        .previous(send.hash())
        .balance(DEV_CONSTANTS.genesis_amount - Amount::new(1))
        .link(send.hash())
        .sign(&DEV_GENESIS_KEY)
        .build()
        .unwrap();

    let result = ctx
        .ledger
        .process(txn.as_mut(), &mut receive, SignatureVerification::Unknown);

    assert_eq!(result.code, ProcessResult::BalanceMismatch);
}

#[test]
fn no_link_amount_fail() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    let send = ctx.process_state_send(
        txn.as_mut(),
        &DEV_GENESIS_KEY,
        *DEV_GENESIS_ACCOUNT,
        Amount::new(1),
    );

    let mut receive = BlockBuilder::state()
        .account(*DEV_GENESIS_ACCOUNT)
        .previous(send.hash())
        .balance(DEV_CONSTANTS.genesis_amount)
        .link(Link::zero())
        .sign(&DEV_GENESIS_KEY)
        .build()
        .unwrap();

    let result = ctx
        .ledger
        .process(txn.as_mut(), &mut receive, SignatureVerification::Unknown);

    assert_eq!(result.code, ProcessResult::BalanceMismatch);
}

#[test]
fn receive_wrong_account_fail() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    let send = ctx.process_state_send(
        txn.as_mut(),
        &DEV_GENESIS_KEY,
        *DEV_GENESIS_ACCOUNT,
        Amount::new(1),
    );

    let key = KeyPair::new();
    let mut receive = BlockBuilder::state()
        .account(key.public_key())
        .previous(BlockHash::zero())
        .balance(Amount::new(1))
        .link(send.hash())
        .sign(&key)
        .build()
        .unwrap();

    let result = ctx
        .ledger
        .process(txn.as_mut(), &mut receive, SignatureVerification::Unknown);

    assert_eq!(result.code, ProcessResult::Unreceivable);
}

#[test]
fn receive_and_change_representative() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    let amount_sent = Amount::new(50);
    let send = ctx.process_state_send(
        txn.as_mut(),
        &DEV_GENESIS_KEY,
        *DEV_GENESIS_ACCOUNT,
        amount_sent,
    );

    let representative = Account::from(1);
    let mut receive = BlockBuilder::state()
        .account(*DEV_GENESIS_ACCOUNT)
        .previous(send.hash())
        .balance(DEV_CONSTANTS.genesis_amount)
        .link(send.hash())
        .representative(representative)
        .sign(&DEV_GENESIS_KEY)
        .build()
        .unwrap();

    ctx.process(txn.as_mut(), &mut receive);

    assert_eq!(
        ctx.ledger.balance(txn.txn(), &receive.hash()),
        receive.balance()
    );
    assert_eq!(
        ctx.ledger.amount(txn.txn(), &receive.hash()).unwrap(),
        amount_sent,
    );
    assert_eq!(ctx.ledger.weight(&DEV_GENESIS_ACCOUNT), Amount::zero());
    assert_eq!(ctx.ledger.weight(&representative), receive.balance());
    assert_eq!(
        receive.sideband().unwrap().details,
        BlockDetails::new(Epoch::Epoch0, false, true, false)
    );
}

fn receive_50_raw_into_genesis(
    ctx: &LedgerContext,
    txn: &mut dyn WriteTransaction,
) -> (StateBlock, StateBlock) {
    let send = ctx.process_state_send(txn, &DEV_GENESIS_KEY, *DEV_GENESIS_ACCOUNT, Amount::new(50));
    let receive = ctx.process_state_receive(txn, &send, &DEV_GENESIS_KEY);
    (send, receive)
}
