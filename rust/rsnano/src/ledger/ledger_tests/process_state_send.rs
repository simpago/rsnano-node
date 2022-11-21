use crate::{
    core::{
        Account, Amount, Block, BlockBuilder, BlockDetails, BlockEnum, Epoch, PendingInfo,
        PendingKey,
    },
    ledger::{ledger_tests::LedgerContext, DEV_GENESIS_KEY},
    DEV_CONSTANTS, DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH,
};

#[test]
fn save_block() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    let receiver_account = Account::from(1);
    let amount_sent = Amount::new(1);
    let block = ctx.process_state_send(
        txn.as_mut(),
        &DEV_GENESIS_KEY,
        receiver_account,
        amount_sent,
    );

    let BlockEnum::State(loaded_block) = ctx.ledger.store.block().get(txn.txn(), &block.hash()).unwrap() else {panic!("not a state block")};
    assert_eq!(loaded_block.sideband().unwrap(), block.sideband().unwrap());
    assert_eq!(loaded_block, block);
    assert_eq!(
        ctx.ledger.amount(txn.txn(), &block.hash()),
        Some(amount_sent)
    );
}

#[test]
fn update_pending_store() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    let receiver_account = Account::from(1);
    let amount_sent = Amount::new(1);
    let block = ctx.process_state_send(
        txn.as_mut(),
        &DEV_GENESIS_KEY,
        receiver_account,
        amount_sent,
    );

    let pending_info = ctx
        .ledger
        .store
        .pending()
        .get(txn.txn(), &PendingKey::new(receiver_account, block.hash()))
        .unwrap();

    assert_eq!(
        pending_info,
        PendingInfo {
            source: block.account(),
            amount: amount_sent,
            epoch: Epoch::Epoch0
        }
    );
}

#[test]
fn create_sideband() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    let receiver_account = Account::from(1);
    let amount_sent = Amount::new(1);
    let block = ctx.process_state_send(
        txn.as_mut(),
        &DEV_GENESIS_KEY,
        receiver_account,
        amount_sent,
    );

    let sideband = block.sideband().unwrap();
    assert_eq!(sideband.height, 2);
    assert_eq!(sideband.account, *DEV_GENESIS_ACCOUNT);
    assert_eq!(
        sideband.details,
        BlockDetails::new(Epoch::Epoch0, true, false, false)
    );
}

#[test]
fn send_and_change_representative() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    let representative = Account::from(1);
    let mut send = BlockBuilder::state()
        .account(*DEV_GENESIS_ACCOUNT)
        .previous(*DEV_GENESIS_HASH)
        .representative(representative)
        .balance(Amount::new(1))
        .link(*DEV_GENESIS_ACCOUNT)
        .sign(&DEV_GENESIS_KEY)
        .build()
        .unwrap();

    ctx.process(txn.as_mut(), &mut send);

    let amount_sent = DEV_CONSTANTS.genesis_amount - Amount::new(1);
    assert_eq!(
        ctx.ledger.amount(txn.txn(), &send.hash()).unwrap(),
        amount_sent,
    );
    assert_eq!(ctx.ledger.weight(&DEV_GENESIS_ACCOUNT), Amount::zero());
    assert_eq!(ctx.ledger.weight(&representative), Amount::new(1));
    assert_eq!(
        send.sideband().unwrap().details,
        BlockDetails::new(Epoch::Epoch0, true, false, false)
    );
}
