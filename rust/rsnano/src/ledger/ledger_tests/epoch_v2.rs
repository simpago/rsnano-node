use crate::{
    core::{
        Account, Amount, Block, BlockBuilder, BlockDetails, Epoch, KeyPair, SignatureVerification,
        StateBlockBuilder,
    },
    ledger::{ProcessResult, DEV_GENESIS_KEY},
    DEV_CONSTANTS, DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH,
};

use super::{epoch_v1::epoch_v1_block_for_genesis_account, LedgerContext};

pub(crate) fn epoch_v2_block_for_genesis_account() -> StateBlockBuilder {
    BlockBuilder::state()
        .account(*DEV_GENESIS_ACCOUNT)
        .previous(*DEV_GENESIS_HASH)
        .representative(*DEV_GENESIS_ACCOUNT)
        .balance(DEV_CONSTANTS.genesis_amount)
        .link(*DEV_CONSTANTS.epochs.link(Epoch::Epoch2).unwrap())
        .sign(&DEV_GENESIS_KEY)
}

#[test]
fn upgrade_from_v0_to_v2_fails() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    let mut epoch = epoch_v2_block_for_genesis_account().build().unwrap();

    let result = ctx
        .ledger
        .process(txn.as_mut(), &mut epoch, SignatureVerification::Unknown);

    // Trying to upgrade from epoch 0 to epoch 2. It is a requirement epoch upgrades are sequential unless the account is unopened
    assert_eq!(result.code, ProcessResult::BlockPosition);
}

#[test]
fn upgrade_to_epoch_v2() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    let mut epoch1 = epoch_v1_block_for_genesis_account().build().unwrap();
    ctx.process(txn.as_mut(), &mut epoch1);

    let mut epoch2 = epoch_v2_block_for_genesis_account()
        .previous(epoch1.hash())
        .build()
        .unwrap();

    let result = ctx
        .ledger
        .process(txn.as_mut(), &mut epoch2, SignatureVerification::Unknown);

    assert_eq!(result.code, ProcessResult::Progress);

    assert_eq!(
        epoch2.sideband().unwrap().details,
        BlockDetails::new(Epoch::Epoch2, false, false, true)
    );
    // source_epoch is not used for send blocks
    assert_eq!(epoch2.sideband().unwrap().source_epoch, Epoch::Epoch0);

    let account_info = ctx
        .ledger
        .store
        .account()
        .get(txn.txn(), &DEV_GENESIS_ACCOUNT)
        .unwrap();
    assert_eq!(account_info.epoch, Epoch::Epoch2);
}

#[test]
fn upgrading_to_epoch_v2_twice_fails() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    let mut epoch1 = epoch_v1_block_for_genesis_account().build().unwrap();
    ctx.process(txn.as_mut(), &mut epoch1);

    let mut epoch2 = epoch_v2_block_for_genesis_account()
        .previous(epoch1.hash())
        .build()
        .unwrap();

    ctx.process(txn.as_mut(), &mut epoch2);

    let mut epoch3 = epoch_v2_block_for_genesis_account()
        .previous(epoch2.hash())
        .build()
        .unwrap();

    let result = ctx
        .ledger
        .process(txn.as_mut(), &mut epoch3, SignatureVerification::Unknown);

    assert_eq!(result.code, ProcessResult::BlockPosition);
}

#[test]
fn rollback_epoch_v2() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();

    let mut epoch1 = epoch_v1_block_for_genesis_account().build().unwrap();
    ctx.process(txn.as_mut(), &mut epoch1);

    let mut epoch2 = epoch_v2_block_for_genesis_account()
        .previous(epoch1.hash())
        .build()
        .unwrap();

    ctx.process(txn.as_mut(), &mut epoch2);

    ctx.ledger
        .rollback(txn.as_mut(), &epoch2.hash(), &mut Vec::new())
        .unwrap();

    let genesis_info = ctx
        .ledger
        .store
        .account()
        .get(txn.txn(), &DEV_GENESIS_ACCOUNT)
        .unwrap();
    assert_eq!(genesis_info.epoch, Epoch::Epoch1);

    let mut old_change = BlockBuilder::change()
        .previous(epoch1.hash())
        .representative(*DEV_GENESIS_ACCOUNT)
        .sign(&DEV_GENESIS_KEY)
        .build()
        .unwrap();

    let result = ctx.ledger.process(
        txn.as_mut(),
        &mut old_change,
        SignatureVerification::Unknown,
    );

    assert_eq!(result.code, ProcessResult::BlockPosition);
}

#[test]
fn upgrade_epoch_after_state_block() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();
    let destination = KeyPair::new();
    let destination_account = Account::from(destination.public_key());

    let mut send = BlockBuilder::state()
        .account(*DEV_GENESIS_ACCOUNT)
        .previous(*DEV_GENESIS_HASH)
        .representative(*DEV_GENESIS_ACCOUNT)
        .balance(DEV_CONSTANTS.genesis_amount - Amount::new(50))
        .link(destination_account)
        .sign(&DEV_GENESIS_KEY)
        .build()
        .unwrap();

    ctx.process(txn.as_mut(), &mut send);

    let mut state_open = BlockBuilder::state()
        .account(destination_account)
        .previous(0)
        .representative(0)
        .balance(0)
        .link(ctx.ledger.epoch_link(Epoch::Epoch1).unwrap())
        .sign(&DEV_GENESIS_KEY)
        .build()
        .unwrap();

    ctx.process(txn.as_mut(), &mut state_open);

    let mut epoch = BlockBuilder::state()
        .account(destination_account)
        .previous(state_open.hash())
        .representative(0)
        .balance(0)
        .link(ctx.ledger.epoch_link(Epoch::Epoch2).unwrap())
        .sign(&DEV_GENESIS_KEY)
        .build()
        .unwrap();

    ctx.process(txn.as_mut(), &mut epoch);
    assert_eq!(
        epoch.sideband().unwrap().details,
        BlockDetails::new(Epoch::Epoch2, false, false, true)
    );
    // source_epoch is not used for send blocks
    assert_eq!(epoch.sideband().unwrap().source_epoch, Epoch::Epoch0);
}

#[test]
fn old_receive_block_after_epoch_v2_upgrade_fails() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();
    let destination = KeyPair::new();
    let destination_account = Account::from(destination.public_key());

    let mut send = BlockBuilder::state()
        .account(*DEV_GENESIS_ACCOUNT)
        .previous(*DEV_GENESIS_HASH)
        .representative(*DEV_GENESIS_ACCOUNT)
        .balance(DEV_CONSTANTS.genesis_amount - Amount::new(50))
        .link(destination_account)
        .sign(&DEV_GENESIS_KEY)
        .build()
        .unwrap();

    ctx.process(txn.as_mut(), &mut send);

    let mut epoch1 = BlockBuilder::state()
        .account(destination_account)
        .previous(0)
        .representative(0)
        .balance(0)
        .link(ctx.ledger.epoch_link(Epoch::Epoch1).unwrap())
        .sign(&DEV_GENESIS_KEY)
        .build()
        .unwrap();

    ctx.process(txn.as_mut(), &mut epoch1);

    let mut epoch2 = BlockBuilder::state()
        .account(destination_account)
        .previous(epoch1.hash())
        .representative(0)
        .balance(0)
        .link(ctx.ledger.epoch_link(Epoch::Epoch2).unwrap())
        .sign(&DEV_GENESIS_KEY)
        .build()
        .unwrap();

    ctx.process(txn.as_mut(), &mut epoch2);

    let mut old_receive = BlockBuilder::receive()
        .previous(epoch2.hash())
        .source(send.hash())
        .sign(&destination)
        .build()
        .unwrap();

    let result = ctx.ledger.process(
        txn.as_mut(),
        &mut old_receive,
        SignatureVerification::Unknown,
    );

    assert_eq!(result.code, ProcessResult::BlockPosition);
}

#[test]
fn cannot_use_old_open_block_with_epoch_v2_send() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();
    let destination = KeyPair::new();
    let destination_account = Account::from(destination.public_key());

    let mut epoch1 = BlockBuilder::state()
        .account(*DEV_GENESIS_ACCOUNT)
        .previous(*DEV_GENESIS_HASH)
        .representative(*DEV_GENESIS_ACCOUNT)
        .balance(DEV_CONSTANTS.genesis_amount)
        .link(ctx.ledger.epoch_link(Epoch::Epoch1).unwrap())
        .sign(&DEV_GENESIS_KEY)
        .build()
        .unwrap();
    ctx.process(txn.as_mut(), &mut epoch1);

    let mut epoch2 = BlockBuilder::state()
        .account(*DEV_GENESIS_ACCOUNT)
        .previous(epoch1.hash())
        .representative(*DEV_GENESIS_ACCOUNT)
        .balance(DEV_CONSTANTS.genesis_amount)
        .link(ctx.ledger.epoch_link(Epoch::Epoch2).unwrap())
        .sign(&DEV_GENESIS_KEY)
        .build()
        .unwrap();
    ctx.process(txn.as_mut(), &mut epoch2);

    // send 50 from genesis to destination
    let mut send = BlockBuilder::state()
        .account(*DEV_GENESIS_ACCOUNT)
        .previous(epoch2.hash())
        .representative(*DEV_GENESIS_ACCOUNT)
        .balance(DEV_CONSTANTS.genesis_amount - Amount::new(50))
        .link(destination_account)
        .sign(&DEV_GENESIS_KEY)
        .build()
        .unwrap();
    ctx.process(txn.as_mut(), &mut send);

    // Try to create an open block from an epoch 2 source block.
    let mut old_open = BlockBuilder::open()
        .source(send.hash())
        .representative(destination_account)
        .account(destination_account)
        .sign(&destination)
        .build()
        .unwrap();
    let result = ctx
        .ledger
        .process(txn.as_mut(), &mut old_open, SignatureVerification::Unknown);
    assert_eq!(result.code, ProcessResult::Unreceivable);
}

#[test]
fn receive_after_epoch_v2() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();
    let destination = KeyPair::new();
    let destination_account = Account::from(destination.public_key());

    let mut epoch1 = epoch_v1_block_for_genesis_account().build().unwrap();
    ctx.process(txn.as_mut(), &mut epoch1);

    let mut send = BlockBuilder::state()
        .account(*DEV_GENESIS_ACCOUNT)
        .previous(epoch1.hash())
        .representative(*DEV_GENESIS_ACCOUNT)
        .balance(DEV_CONSTANTS.genesis_amount - Amount::new(50))
        .link(destination_account)
        .sign(&DEV_GENESIS_KEY)
        .build()
        .unwrap();

    ctx.process(txn.as_mut(), &mut send);

    let mut epoch1 = BlockBuilder::state()
        .account(destination_account)
        .previous(0)
        .representative(0)
        .balance(0)
        .link(ctx.ledger.epoch_link(Epoch::Epoch1).unwrap())
        .sign(&DEV_GENESIS_KEY)
        .build()
        .unwrap();

    ctx.process(txn.as_mut(), &mut epoch1);

    let mut epoch2 = BlockBuilder::state()
        .account(destination_account)
        .previous(epoch1.hash())
        .representative(0)
        .balance(0)
        .link(ctx.ledger.epoch_link(Epoch::Epoch2).unwrap())
        .sign(&DEV_GENESIS_KEY)
        .build()
        .unwrap();

    ctx.process(txn.as_mut(), &mut epoch2);

    let mut receive = BlockBuilder::state()
        .account(destination_account)
        .previous(epoch2.hash())
        .representative(destination_account)
        .balance(Amount::new(50))
        .link(send.hash())
        .sign(&destination)
        .build()
        .unwrap();

    ctx.process(txn.as_mut(), &mut receive);
    assert_eq!(
        receive.sideband().unwrap().details,
        BlockDetails::new(Epoch::Epoch2, false, true, false)
    );
    assert_eq!(receive.sideband().unwrap().source_epoch, Epoch::Epoch1);
    assert_eq!(ctx.ledger.weight(&destination_account), Amount::new(50));
}

#[test]
fn receiving_from_epoch_2_block_upgrades_receiver_to_epoch2() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();
    let destination = KeyPair::new();
    let destination_account = Account::from(destination.public_key());

    // send 50 to destination
    let mut send1 = BlockBuilder::state()
        .account(*DEV_GENESIS_ACCOUNT)
        .previous(*DEV_GENESIS_HASH)
        .representative(*DEV_GENESIS_ACCOUNT)
        .balance(DEV_CONSTANTS.genesis_amount - Amount::new(50))
        .link(destination_account)
        .sign(&DEV_GENESIS_KEY)
        .build()
        .unwrap();
    ctx.process(txn.as_mut(), &mut send1);

    // open destination
    let mut open1 = BlockBuilder::open()
        .source(send1.hash())
        .representative(destination_account)
        .account(destination_account)
        .sign(&destination)
        .build()
        .unwrap();
    ctx.process(txn.as_mut(), &mut open1);

    let mut epoch1 = BlockBuilder::state()
        .account(*DEV_GENESIS_ACCOUNT)
        .previous(send1.hash())
        .representative(*DEV_GENESIS_ACCOUNT)
        .balance(DEV_CONSTANTS.genesis_amount - Amount::new(50))
        .link(ctx.ledger.epoch_link(Epoch::Epoch1).unwrap())
        .sign(&DEV_GENESIS_KEY)
        .build()
        .unwrap();
    ctx.process(txn.as_mut(), &mut epoch1);

    let mut epoch2 = BlockBuilder::state()
        .account(*DEV_GENESIS_ACCOUNT)
        .previous(epoch1.hash())
        .representative(*DEV_GENESIS_ACCOUNT)
        .balance(DEV_CONSTANTS.genesis_amount - Amount::new(50))
        .link(ctx.ledger.epoch_link(Epoch::Epoch2).unwrap())
        .sign(&DEV_GENESIS_KEY)
        .build()
        .unwrap();
    ctx.process(txn.as_mut(), &mut epoch2);

    //lend 50 to destination
    let mut send2 = BlockBuilder::state()
        .account(*DEV_GENESIS_ACCOUNT)
        .previous(epoch2.hash())
        .representative(*DEV_GENESIS_ACCOUNT)
        .balance(DEV_CONSTANTS.genesis_amount - Amount::new(100))
        .link(destination_account)
        .sign(&DEV_GENESIS_KEY)
        .build()
        .unwrap();
    ctx.process(txn.as_mut(), &mut send2);

    let mut receive2 = BlockBuilder::state()
        .account(destination_account)
        .previous(open1.hash())
        .representative(destination_account)
        .balance(Amount::new(100))
        .link(send2.hash())
        .sign(&destination)
        .build()
        .unwrap();
    ctx.process(txn.as_mut(), &mut receive2);

    assert_eq!(receive2.sideband().unwrap().details.epoch, Epoch::Epoch2);
    assert_eq!(receive2.sideband().unwrap().source_epoch, Epoch::Epoch2);
    let destination_info = ctx
        .ledger
        .store
        .account()
        .get(txn.txn(), &destination_account)
        .unwrap();
    assert_eq!(destination_info.epoch, Epoch::Epoch2);
}

#[test]
fn upgrade_new_account_straight_to_epoch_2() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();
    let destination = KeyPair::new();
    let destination_account = Account::from(destination.public_key());

    let mut epoch1 = BlockBuilder::state()
        .account(*DEV_GENESIS_ACCOUNT)
        .previous(*DEV_GENESIS_HASH)
        .representative(*DEV_GENESIS_ACCOUNT)
        .balance(DEV_CONSTANTS.genesis_amount)
        .link(ctx.ledger.epoch_link(Epoch::Epoch1).unwrap())
        .sign(&DEV_GENESIS_KEY)
        .build()
        .unwrap();
    ctx.process(txn.as_mut(), &mut epoch1);

    let mut epoch2 = BlockBuilder::state()
        .account(*DEV_GENESIS_ACCOUNT)
        .previous(epoch1.hash())
        .representative(*DEV_GENESIS_ACCOUNT)
        .balance(DEV_CONSTANTS.genesis_amount)
        .link(ctx.ledger.epoch_link(Epoch::Epoch2).unwrap())
        .sign(&DEV_GENESIS_KEY)
        .build()
        .unwrap();
    ctx.process(txn.as_mut(), &mut epoch2);

    //lend 50 to destination
    let mut send = BlockBuilder::state()
        .account(*DEV_GENESIS_ACCOUNT)
        .previous(epoch2.hash())
        .representative(*DEV_GENESIS_ACCOUNT)
        .balance(DEV_CONSTANTS.genesis_amount - Amount::new(50))
        .link(destination_account)
        .sign(&DEV_GENESIS_KEY)
        .build()
        .unwrap();
    ctx.process(txn.as_mut(), &mut send);

    let mut open = BlockBuilder::state()
        .account(destination_account)
        .previous(0)
        .representative(0)
        .balance(0)
        .link(ctx.ledger.epoch_link(Epoch::Epoch2).unwrap())
        .sign(&DEV_GENESIS_KEY)
        .build()
        .unwrap();
    ctx.process(txn.as_mut(), &mut open);
    assert_eq!(open.sideband().unwrap().details.epoch, Epoch::Epoch2);
    assert_eq!(open.sideband().unwrap().source_epoch, Epoch::Epoch0); // Not used for epoch blocks
}
