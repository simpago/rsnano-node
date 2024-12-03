mod account_block_factory;

use crate::LedgerContext;
pub(crate) use account_block_factory::AccountBlockFactory;
use rsnano_core::{Amount, Block, SavedBlock};
use rsnano_store_lmdb::LmdbWriteTransaction;

pub(crate) fn upgrade_genesis_to_epoch_v1(
    ctx: &LedgerContext,
    txn: &mut LmdbWriteTransaction,
) -> Block {
    let mut epoch = ctx.genesis_block_factory().epoch_v1(txn).build();
    ctx.ledger.process(txn, &mut epoch).unwrap();
    epoch
}

pub(crate) struct LegacySendBlockResult<'a> {
    pub destination: AccountBlockFactory<'a>,
    pub send_block: SavedBlock,
    pub amount_sent: Amount,
}
pub(crate) fn setup_legacy_send_block<'a>(
    ctx: &'a LedgerContext,
    txn: &mut LmdbWriteTransaction,
) -> LegacySendBlockResult<'a> {
    let genesis = ctx.genesis_block_factory();
    let destination = ctx.block_factory();

    let amount_sent = Amount::raw(50);
    let mut send_block = genesis
        .legacy_send(txn)
        .destination(destination.account())
        .amount(amount_sent)
        .build();
    let send_block = ctx.ledger.process(txn, &mut send_block).unwrap();
    LegacySendBlockResult {
        destination,
        send_block,
        amount_sent,
    }
}

pub(crate) struct LegacyOpenBlockResult<'a> {
    pub destination: AccountBlockFactory<'a>,
    pub send_block: SavedBlock,
    pub open_block: SavedBlock,
    pub expected_balance: Amount,
}

pub(crate) fn setup_legacy_open_block<'a>(
    ctx: &'a LedgerContext,
    txn: &mut LmdbWriteTransaction,
) -> LegacyOpenBlockResult<'a> {
    let send = setup_legacy_send_block(ctx, txn);

    let mut open_block = send.destination.legacy_open(send.send_block.hash()).build();
    let open_block = ctx.ledger.process(txn, &mut open_block).unwrap();

    LegacyOpenBlockResult {
        destination: send.destination,
        send_block: send.send_block,
        open_block,
        expected_balance: send.amount_sent,
    }
}

pub(crate) struct LegacyReceiveBlockResult<'a> {
    pub destination: AccountBlockFactory<'a>,
    pub open_block: SavedBlock,
    pub send_block: Block,
    pub receive_block: Block,
    pub expected_balance: Amount,
    pub amount_received: Amount,
}
pub(crate) fn setup_legacy_receive_block<'a>(
    ctx: &'a LedgerContext,
    txn: &mut LmdbWriteTransaction,
) -> LegacyReceiveBlockResult<'a> {
    let genesis = ctx.genesis_block_factory();

    let open = setup_legacy_open_block(ctx, txn);

    let amount_sent2 = Amount::raw(25);
    let mut send2 = genesis
        .legacy_send(txn)
        .destination(open.destination.account())
        .amount(amount_sent2)
        .build();
    ctx.ledger.process(txn, &mut send2).unwrap();

    let mut receive_block = open.destination.legacy_receive(txn, send2.hash()).build();
    ctx.ledger.process(txn, &mut receive_block).unwrap();

    LegacyReceiveBlockResult {
        destination: open.destination,
        open_block: open.open_block,
        send_block: send2,
        receive_block,
        expected_balance: open.expected_balance + amount_sent2,
        amount_received: amount_sent2,
    }
}

pub(crate) struct SendBlockResult<'a> {
    pub destination: AccountBlockFactory<'a>,
    pub send_block: SavedBlock,
}
pub(crate) fn setup_send_block<'a>(
    ctx: &'a LedgerContext,
    txn: &mut LmdbWriteTransaction,
) -> SendBlockResult<'a> {
    let genesis = ctx.genesis_block_factory();
    let destination = ctx.block_factory();

    let amount_sent = Amount::raw(50);
    let mut send_block = genesis
        .send(txn)
        .link(destination.account())
        .amount_sent(amount_sent)
        .build();
    let send_block = ctx.ledger.process(txn, &mut send_block).unwrap();

    SendBlockResult {
        destination,
        send_block,
    }
}

pub(crate) struct OpenBlockResult {
    pub send_block: SavedBlock,
    pub open_block: SavedBlock,
}
pub(crate) fn setup_open_block(
    ctx: &LedgerContext,
    txn: &mut LmdbWriteTransaction,
) -> OpenBlockResult {
    let send = setup_send_block(ctx, txn);

    let mut open_block = send.destination.open(txn, send.send_block.hash()).build();
    let open_block = ctx.ledger.process(txn, &mut open_block).unwrap();

    OpenBlockResult {
        send_block: send.send_block,
        open_block,
    }
}
