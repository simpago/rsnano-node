mod ledger_context;
use std::{ops::Deref, sync::atomic::Ordering};

pub(crate) use ledger_context::LedgerContext;

mod test_contexts;
pub(crate) use test_contexts::*;

use crate::{
    core::{
        Account, Amount, Block, BlockBuilder, BlockEnum, BlockHash, KeyPair, QualifiedRoot, Root,
        GXRB_RATIO,
    },
    DEV_CONSTANTS, DEV_GENESIS, DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH,
};

use super::DEV_GENESIS_KEY;

mod empty_ledger;
mod epoch_blocks;
mod process_change;
mod process_open;
mod process_receive;
mod process_send;
mod process_state_change;
mod process_state_open;
mod process_state_receive;
mod process_state_send;
mod rollback_change;
mod rollback_open;
mod rollback_receive;
mod rollback_send;
mod rollback_state;

#[test]
fn ledger_successor() {
    let ctx = LedgerWithSendBlock::new();
    assert_eq!(
        ctx.ledger().successor(
            ctx.txn.txn(),
            &QualifiedRoot::new(Root::zero(), *DEV_GENESIS_HASH)
        ),
        Some(BlockEnum::Send(ctx.send_block))
    );
}

#[test]
fn ledger_successor_genesis() {
    let ctx = LedgerWithSendBlock::new();
    let genesis = DEV_GENESIS.read().unwrap().clone();
    assert_eq!(
        ctx.ledger().successor(
            ctx.txn.txn(),
            &QualifiedRoot::new(DEV_GENESIS_ACCOUNT.deref().into(), BlockHash::zero())
        ),
        Some(genesis)
    );
}

#[test]
fn latest_root_empty() {
    let ctx = LedgerContext::empty();
    let txn = ctx.ledger.read_txn();
    assert_eq!(
        ctx.ledger.latest_root(txn.txn(), &Account::from(1)),
        Root::from(1)
    );
}

#[test]
fn latest_root() {
    let ctx = LedgerWithSendBlock::new();

    assert_eq!(
        ctx.ledger()
            .latest_root(ctx.txn.txn(), &DEV_GENESIS_ACCOUNT),
        ctx.send_block.hash().into()
    );
}

#[test]
fn send_open_receive_vote_weight() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();
    let receiver_key = KeyPair::new();
    let receiver_account = receiver_key.public_key().into();
    let send1 = ctx.process_send_from_genesis(txn.as_mut(), &receiver_account, Amount::new(50));
    let send2 = ctx.process_send_from_genesis(txn.as_mut(), &receiver_account, Amount::new(50));
    ctx.process_open(txn.as_mut(), &send1, &receiver_key);
    ctx.process_receive(txn.as_mut(), &send2, &receiver_key);

    assert_eq!(ctx.ledger.weight(&receiver_account), Amount::new(100));
    assert_eq!(
        ctx.ledger.weight(&DEV_GENESIS_ACCOUNT),
        DEV_CONSTANTS.genesis_amount - Amount::new(100)
    );
}

#[test]
fn send_open_receive_rollback() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();
    let receiver_key = KeyPair::new();
    let receiver_account = receiver_key.public_key().into();
    let send1 = ctx.process_send_from_genesis(txn.as_mut(), &receiver_account, Amount::new(50));
    let send2 = ctx.process_send_from_genesis(txn.as_mut(), &receiver_account, Amount::new(50));
    let open = ctx.process_open(txn.as_mut(), &send1, &receiver_key);
    let receive = ctx.process_receive(txn.as_mut(), &send2, &receiver_key);
    let rep_account = Account::from(1);
    let change = ctx.process_change(txn.as_mut(), &DEV_GENESIS_KEY, rep_account);

    ctx.ledger
        .rollback(txn.as_mut(), &receive.hash(), &mut Vec::new())
        .unwrap();

    assert_eq!(ctx.ledger.weight(&receiver_account), Amount::new(50));
    assert_eq!(ctx.ledger.weight(&DEV_GENESIS_ACCOUNT), Amount::zero());
    assert_eq!(
        ctx.ledger.weight(&rep_account),
        DEV_CONSTANTS.genesis_amount - Amount::new(100)
    );

    ctx.ledger
        .rollback(txn.as_mut(), &open.hash(), &mut Vec::new())
        .unwrap();

    assert_eq!(ctx.ledger.weight(&receiver_account), Amount::zero());
    assert_eq!(ctx.ledger.weight(&DEV_GENESIS_ACCOUNT), Amount::zero());
    assert_eq!(
        ctx.ledger.weight(&rep_account),
        DEV_CONSTANTS.genesis_amount - Amount::new(100)
    );

    ctx.ledger
        .rollback(txn.as_mut(), &change.hash(), &mut Vec::new())
        .unwrap();

    assert_eq!(ctx.ledger.weight(&receiver_account), Amount::zero());
    assert_eq!(ctx.ledger.weight(&rep_account), Amount::zero());
    assert_eq!(
        ctx.ledger.weight(&DEV_GENESIS_ACCOUNT),
        DEV_CONSTANTS.genesis_amount - Amount::new(100)
    );

    ctx.ledger
        .rollback(txn.as_mut(), &send2.hash(), &mut Vec::new())
        .unwrap();

    assert_eq!(ctx.ledger.weight(&receiver_account), Amount::zero());
    assert_eq!(ctx.ledger.weight(&rep_account), Amount::zero());
    assert_eq!(
        ctx.ledger.weight(&DEV_GENESIS_ACCOUNT),
        DEV_CONSTANTS.genesis_amount - Amount::new(50)
    );

    ctx.ledger
        .rollback(txn.as_mut(), &send1.hash(), &mut Vec::new())
        .unwrap();

    assert_eq!(ctx.ledger.weight(&receiver_account), Amount::zero());
    assert_eq!(ctx.ledger.weight(&rep_account), Amount::zero());
    assert_eq!(
        ctx.ledger.weight(&DEV_GENESIS_ACCOUNT),
        DEV_CONSTANTS.genesis_amount
    );
}

#[test]
fn bootstrap_rep_weight() {
    let ctx = LedgerContext::empty();
    ctx.ledger.set_bootstrap_weight_max_blocks(3);
    let representative_key = KeyPair::new();
    let representative_account = representative_key.public_key().into();
    {
        let mut txn = ctx.ledger.rw_txn();
        ctx.process_send_from_genesis(txn.as_mut(), &representative_account, Amount::new(50));
    }
    {
        let mut weights = ctx.ledger.bootstrap_weights.lock().unwrap();
        weights.insert(representative_account, Amount::new(1000));
    }
    assert_eq!(ctx.ledger.cache.block_count.load(Ordering::Relaxed), 2);
    assert_eq!(
        ctx.ledger.weight(&representative_account),
        Amount::new(1000)
    );
    {
        let mut txn = ctx.ledger.rw_txn();
        ctx.process_send_from_genesis(txn.as_mut(), &representative_account, Amount::new(50));
    }
    assert_eq!(ctx.ledger.cache.block_count.load(Ordering::Relaxed), 3);
    assert_eq!(ctx.ledger.weight(&representative_account), Amount::zero());
}

#[test]
fn block_destination_source() {
    let ctx = LedgerContext::empty();
    let ledger = &ctx.ledger;
    let mut txn = ledger.rw_txn();
    let dest = KeyPair::new();
    let dest_account = dest.public_key().into();
    let block1 =
        ctx.process_send_from_genesis(txn.as_mut(), &dest_account, Amount::new(*GXRB_RATIO));
    let block2 =
        ctx.process_send_from_genesis(txn.as_mut(), &DEV_GENESIS_ACCOUNT, Amount::new(*GXRB_RATIO));
    let block3 = ctx.process_receive(txn.as_mut(), &block2, &DEV_GENESIS_KEY);

    let mut block4 = BlockBuilder::state()
        .account(*DEV_GENESIS_ACCOUNT)
        .previous(block3.hash())
        .balance(DEV_CONSTANTS.genesis_amount - Amount::new(*GXRB_RATIO * 2))
        .link(dest_account)
        .sign(&DEV_GENESIS_KEY)
        .build()
        .unwrap();
    ctx.process(txn.as_mut(), &mut block4);

    let mut block5 = BlockBuilder::state()
        .account(*DEV_GENESIS_ACCOUNT)
        .previous(block4.hash())
        .balance(DEV_CONSTANTS.genesis_amount - Amount::new(*GXRB_RATIO * 3))
        .link(*DEV_GENESIS_ACCOUNT)
        .sign(&DEV_GENESIS_KEY)
        .build()
        .unwrap();
    ctx.process(txn.as_mut(), &mut block5);

    let mut block6 = BlockBuilder::state()
        .account(*DEV_GENESIS_ACCOUNT)
        .previous(block5.hash())
        .balance(DEV_CONSTANTS.genesis_amount - Amount::new(*GXRB_RATIO * 2))
        .link(block5.hash())
        .sign(&DEV_GENESIS_KEY)
        .build()
        .unwrap();
    ctx.process(txn.as_mut(), &mut block6);

    let block1 = BlockEnum::Send(block1);
    let block2 = BlockEnum::Send(block2);
    let block3 = BlockEnum::Receive(block3);
    let block4 = BlockEnum::State(block4);
    let block5 = BlockEnum::State(block5);
    let block6 = BlockEnum::State(block6);

    assert_eq!(
        ledger.balance(txn.txn(), &block6.as_block().hash()),
        block6.as_block().balance()
    );
    assert_eq!(ledger.block_destination(txn.txn(), &block1), dest_account);
    assert_eq!(ledger.block_source(txn.txn(), &block1), BlockHash::zero());

    assert_eq!(
        ledger.block_destination(txn.txn(), &block2),
        *DEV_GENESIS_ACCOUNT
    );
    assert_eq!(ledger.block_source(txn.txn(), &block2), BlockHash::zero());

    assert_eq!(
        ledger.block_destination(txn.txn(), &block3),
        Account::zero()
    );
    assert_eq!(
        ledger.block_source(txn.txn(), &block3),
        block2.as_block().hash()
    );

    assert_eq!(ledger.block_destination(txn.txn(), &block4), dest_account);
    assert_eq!(ledger.block_source(txn.txn(), &block4), BlockHash::zero());

    assert_eq!(
        ledger.block_destination(txn.txn(), &block5),
        *DEV_GENESIS_ACCOUNT
    );
    assert_eq!(ledger.block_source(txn.txn(), &block5), BlockHash::zero());

    assert_eq!(
        ledger.block_destination(txn.txn(), &block6),
        Account::zero()
    );
    assert_eq!(
        ledger.block_source(txn.txn(), &block6),
        block5.as_block().hash()
    );
}

#[test]
fn state_account() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();
    let mut send = BlockBuilder::state()
        .account(*DEV_GENESIS_ACCOUNT)
        .previous(*DEV_GENESIS_HASH)
        .balance(DEV_CONSTANTS.genesis_amount - Amount::new(*GXRB_RATIO))
        .link(*DEV_GENESIS_ACCOUNT)
        .sign(&DEV_GENESIS_KEY)
        .build()
        .unwrap();
    ctx.process(txn.as_mut(), &mut send);
    assert_eq!(
        ctx.ledger.account(txn.txn(), &send.hash()),
        Some(*DEV_GENESIS_ACCOUNT)
    );
}
