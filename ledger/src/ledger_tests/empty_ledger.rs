use super::LedgerContext;
use crate::{
    ledger_constants::{DEV_GENESIS_PUB_KEY, LEDGER_CONSTANTS_STUB},
    DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH,
};
use rsnano_core::{utils::UnixTimestamp, Account, BlockType};

#[test]
fn account_balance_is_none_for_unknown_account() {
    let ctx = LedgerContext::empty();
    let txn = ctx.ledger.read_txn();

    let balance = ctx.ledger.any().account_balance(&txn, &Account::zero());

    assert_eq!(balance, None);
}

#[test]
fn get_genesis_block() {
    let ctx = LedgerContext::empty();
    let txn = ctx.ledger.read_txn();

    let block = ctx
        .ledger
        .any()
        .get_block(&txn, &DEV_GENESIS_HASH)
        .expect("genesis block not found");

    assert_eq!(block.block_type(), BlockType::LegacyOpen);
}

#[test]
fn genesis_account_balance() {
    let ctx = LedgerContext::empty();
    let txn = ctx.ledger.read_txn();

    let balance = ctx.ledger.any().account_balance(&txn, &DEV_GENESIS_ACCOUNT);

    assert_eq!(balance, Some(LEDGER_CONSTANTS_STUB.genesis_amount));
}

#[test]
fn genesis_account_info() {
    let ctx = LedgerContext::empty();
    let txn = ctx.ledger.read_txn();

    let account_info = ctx
        .ledger
        .account_info(&txn, &DEV_GENESIS_ACCOUNT)
        .expect("genesis account not found");

    // Frontier time should have been updated when genesis balance was added
    assert_eq!(account_info.modified, UnixTimestamp::ZERO);
    assert_eq!(account_info.block_count, 1);
    assert_eq!(account_info.balance, LEDGER_CONSTANTS_STUB.genesis_amount);
}

#[test]
fn genesis_confirmation_height_info() {
    let ctx = LedgerContext::empty();
    let txn = ctx.ledger.read_txn();

    // Genesis block should be confirmed by default
    let conf_info = ctx
        .ledger
        .get_confirmation_height(&txn, &DEV_GENESIS_ACCOUNT)
        .expect("conf height not found");

    assert_eq!(conf_info.height, 1);
    assert_eq!(conf_info.frontier, *DEV_GENESIS_HASH);
}

#[test]
fn cache() {
    let ctx = LedgerContext::empty();
    assert_eq!(ctx.ledger.account_count(), 1);
    assert_eq!(ctx.ledger.cemented_count(), 1);
}

#[test]
fn genesis_representative() {
    let ctx = LedgerContext::empty();
    let txn = ctx.ledger.read_txn();
    assert_eq!(
        ctx.ledger
            .representative_block_hash(&txn, &DEV_GENESIS_HASH),
        *DEV_GENESIS_HASH
    );
}

#[test]
fn genesis_vote_weight() {
    let ctx = LedgerContext::empty();
    assert_eq!(
        ctx.ledger.weight(&DEV_GENESIS_PUB_KEY),
        LEDGER_CONSTANTS_STUB.genesis_amount
    );
}

#[test]
fn latest_empty() {
    let ctx = LedgerContext::empty();
    let txn = ctx.ledger.read_txn();
    assert_eq!(ctx.ledger.any().account_head(&txn, &Account::from(1)), None);
}
