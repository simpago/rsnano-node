use std::sync::{atomic::Ordering, Arc};
pub mod helpers;
use crate::{
    ledger_constants::{DEV_GENESIS_BLOCK, DEV_GENESIS_PUB_KEY, LEDGER_CONSTANTS_STUB},
    ledger_tests::helpers::{setup_legacy_open_block, setup_open_block, AccountBlockFactory},
    Ledger, LedgerContext, RepWeightCache, DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH,
};
use rsnano_core::{
    utils::{new_test_timestamp, TEST_ENDPOINT_1},
    Account, Amount, BlockBuilder, BlockHash, PublicKey, QualifiedRoot, Root, SavedAccountChain,
    DEV_GENESIS_KEY,
};

mod empty_ledger;
mod pruning;
mod receivable_iteration;
mod rollback_legacy_change;
mod rollback_legacy_receive;
mod rollback_legacy_send;
mod rollback_state;

#[test]
fn ledger_successor() {
    let mut chain = SavedAccountChain::new_opened_chain();
    let send = chain.add_legacy_send().clone();
    let ledger = Ledger::new_null_builder()
        .blocks(chain.blocks())
        .account_info(&chain.account(), &chain.account_info())
        .finish();
    let txn = ledger.read_txn();

    assert_eq!(
        ledger.any().block_successor_by_qualified_root(
            &txn,
            &QualifiedRoot::new(Root::zero(), chain.open())
        ),
        Some(send.hash())
    );
}

#[test]
fn ledger_successor_genesis() {
    let mut genesis = SavedAccountChain::genesis();
    genesis.add_legacy_send();
    let ledger = Ledger::new_null_builder()
        .blocks(genesis.blocks())
        .account_info(&genesis.account(), &genesis.account_info())
        .finish();
    let txn = ledger.read_txn();

    assert_eq!(
        ledger.any().block_successor_by_qualified_root(
            &txn,
            &QualifiedRoot::new(genesis.account().into(), BlockHash::zero())
        ),
        Some(genesis.block(1).hash())
    );
}

#[test]
fn latest_root_empty() {
    let ledger = Ledger::new_null();
    let txn = ledger.read_txn();
    assert_eq!(ledger.latest_root(&txn, &Account::from(1)), Root::from(1));
}

#[test]
fn latest_root() {
    let mut genesis = SavedAccountChain::genesis();
    genesis.add_legacy_send();

    let ledger = Ledger::new_null_builder()
        .blocks(genesis.blocks())
        .account_info(&genesis.account(), &genesis.account_info())
        .finish();
    let txn = ledger.rw_txn();

    assert_eq!(
        ledger.latest_root(&txn, &DEV_GENESIS_ACCOUNT),
        genesis.frontier().into()
    );
}

#[test]
fn send_open_receive_vote_weight() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();
    let genesis = ctx.genesis_block_factory();
    let receiver = ctx.block_factory();

    let mut send1 = genesis
        .legacy_send(&txn)
        .destination(receiver.account())
        .amount(Amount::raw(50))
        .build();
    ctx.ledger.process(&mut txn, &mut send1).unwrap();

    let mut send2 = genesis
        .legacy_send(&txn)
        .destination(receiver.account())
        .amount(Amount::raw(50))
        .build();
    ctx.ledger.process(&mut txn, &mut send2).unwrap();

    let mut open = receiver.legacy_open(send1.hash()).build();
    ctx.ledger.process(&mut txn, &mut open).unwrap();

    let mut receive = receiver.legacy_receive(&txn, send2.hash()).build();
    ctx.ledger.process(&mut txn, &mut receive).unwrap();

    assert_eq!(ctx.ledger.weight(&receiver.public_key()), Amount::raw(100));
    assert_eq!(
        ctx.ledger.weight(&DEV_GENESIS_PUB_KEY),
        LEDGER_CONSTANTS_STUB.genesis_amount - Amount::raw(100)
    );
}

#[test]
fn send_open_receive_rollback() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();
    let genesis = ctx.genesis_block_factory();
    let receiver = AccountBlockFactory::new(&ctx.ledger);

    let mut send1 = genesis
        .legacy_send(&txn)
        .destination(receiver.account())
        .amount(Amount::raw(50))
        .build();
    ctx.ledger.process(&mut txn, &mut send1).unwrap();

    let mut send2 = genesis
        .legacy_send(&txn)
        .destination(receiver.account())
        .amount(Amount::raw(50))
        .build();
    ctx.ledger.process(&mut txn, &mut send2).unwrap();

    let mut open = receiver.legacy_open(send1.hash()).build();
    ctx.ledger.process(&mut txn, &mut open).unwrap();

    let mut receive = receiver.legacy_receive(&txn, send2.hash()).build();
    ctx.ledger.process(&mut txn, &mut receive).unwrap();
    let rep_account = PublicKey::from(1);

    let mut change = genesis
        .legacy_change(&txn)
        .representative(rep_account)
        .build();
    ctx.ledger.process(&mut txn, &mut change).unwrap();

    ctx.ledger.rollback(&mut txn, &receive.hash()).unwrap();

    assert_eq!(ctx.ledger.weight(&receiver.public_key()), Amount::raw(50));
    assert_eq!(ctx.ledger.weight(&DEV_GENESIS_PUB_KEY), Amount::zero());
    assert_eq!(
        ctx.ledger.weight(&rep_account),
        LEDGER_CONSTANTS_STUB.genesis_amount - Amount::raw(100)
    );

    ctx.ledger.rollback(&mut txn, &open.hash()).unwrap();

    assert_eq!(ctx.ledger.weight(&receiver.public_key()), Amount::zero());
    assert_eq!(ctx.ledger.weight(&DEV_GENESIS_PUB_KEY), Amount::zero());
    assert_eq!(
        ctx.ledger.weight(&rep_account),
        LEDGER_CONSTANTS_STUB.genesis_amount - Amount::raw(100)
    );

    ctx.ledger.rollback(&mut txn, &change.hash()).unwrap();

    assert_eq!(ctx.ledger.weight(&receiver.public_key()), Amount::zero());
    assert_eq!(ctx.ledger.weight(&rep_account), Amount::zero());
    assert_eq!(
        ctx.ledger.weight(&DEV_GENESIS_PUB_KEY),
        LEDGER_CONSTANTS_STUB.genesis_amount - Amount::raw(100)
    );

    ctx.ledger.rollback(&mut txn, &send2.hash()).unwrap();

    assert_eq!(ctx.ledger.weight(&receiver.public_key()), Amount::zero());
    assert_eq!(ctx.ledger.weight(&rep_account), Amount::zero());
    assert_eq!(
        ctx.ledger.weight(&DEV_GENESIS_PUB_KEY),
        LEDGER_CONSTANTS_STUB.genesis_amount - Amount::raw(50)
    );

    ctx.ledger.rollback(&mut txn, &send1.hash()).unwrap();

    assert_eq!(ctx.ledger.weight(&receiver.public_key()), Amount::zero());
    assert_eq!(ctx.ledger.weight(&rep_account), Amount::zero());
    assert_eq!(
        ctx.ledger.weight(&DEV_GENESIS_PUB_KEY),
        LEDGER_CONSTANTS_STUB.genesis_amount
    );
}

#[test]
fn block_destination_source() {
    let ctx = LedgerContext::empty();
    let ledger = &ctx.ledger;
    let mut txn = ledger.rw_txn();
    let genesis = ctx.genesis_block_factory();
    let dest_account = Account::from(1000);

    let mut send_to_dest = genesis.legacy_send(&txn).destination(dest_account).build();
    let send_to_dest = ctx.ledger.process(&mut txn, &mut send_to_dest).unwrap();

    let mut send_to_self = genesis
        .legacy_send(&txn)
        .destination(genesis.account())
        .build();
    let send_to_self = ctx.ledger.process(&mut txn, &mut send_to_self).unwrap();

    let mut receive = genesis.legacy_receive(&txn, send_to_self.hash()).build();
    let receive = ctx.ledger.process(&mut txn, &mut receive).unwrap();

    let mut send_to_dest_2 = genesis.send(&txn).link(dest_account).build();
    let send_to_dest_2 = ctx.ledger.process(&mut txn, &mut send_to_dest_2).unwrap();

    let mut send_to_self_2 = genesis.send(&txn).link(genesis.account()).build();
    let send_to_self_2 = ctx.ledger.process(&mut txn, &mut send_to_self_2).unwrap();

    let mut receive2 = genesis.receive(&txn, send_to_self_2.hash()).build();
    let receive2 = ctx.ledger.process(&mut txn, &mut receive2).unwrap();

    let block1 = send_to_dest;
    let block2 = send_to_self;
    let block3 = receive;
    let block4 = send_to_dest_2;
    let block5 = send_to_self_2;
    let block6 = receive2;

    assert_eq!(
        ledger.any().block_balance(&txn, &block6.hash()),
        Some(block6.balance_field().unwrap())
    );
    assert_eq!(block1.destination(), Some(dest_account));
    assert_eq!(block1.source(), None);

    assert_eq!(block2.destination(), Some(*DEV_GENESIS_ACCOUNT));
    assert_eq!(block2.source(), None);

    assert_eq!(block3.destination(), None);
    assert_eq!(block3.source(), Some(block2.hash()));

    assert_eq!(block4.destination(), Some(dest_account));
    assert_eq!(block4.source(), None);

    assert_eq!(block5.destination(), Some(*DEV_GENESIS_ACCOUNT));
    assert_eq!(block5.source(), None);

    assert_eq!(block6.destination(), None);
    assert_eq!(block6.source(), Some(block5.hash()));
}

#[test]
fn state_account() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();
    let mut send = BlockBuilder::state()
        .account(*DEV_GENESIS_ACCOUNT)
        .previous(*DEV_GENESIS_HASH)
        .balance(LEDGER_CONSTANTS_STUB.genesis_amount - Amount::nano(1000))
        .link(*DEV_GENESIS_ACCOUNT)
        .sign(&DEV_GENESIS_KEY)
        .build();
    ctx.ledger.process(&mut txn, &mut send).unwrap();
    assert_eq!(
        ctx.ledger.any().block_account(&txn, &send.hash()),
        Some(*DEV_GENESIS_ACCOUNT)
    );
}

mod dependents_confirmed {
    use super::*;
    use crate::ledger_constants::DEV_GENESIS_BLOCK;

    #[test]
    fn genesis_is_confirmed() {
        let ctx = LedgerContext::empty();
        let txn = ctx.ledger.read_txn();

        assert_eq!(
            ctx.ledger
                .dependents_confirmed_for_unsaved_block(&txn, &DEV_GENESIS_BLOCK),
            true
        );
    }

    #[test]
    fn send_dependents_are_confirmed_if_previous_block_is_confirmed() {
        let ctx = LedgerContext::empty();
        let mut txn = ctx.ledger.rw_txn();
        let destination = ctx.block_factory();

        let mut send = ctx
            .genesis_block_factory()
            .send(&txn)
            .link(destination.account())
            .build();
        ctx.ledger.process(&mut txn, &mut send).unwrap();

        assert_eq!(
            ctx.ledger
                .dependents_confirmed_for_unsaved_block(&txn, &send),
            true
        );
    }

    #[test]
    fn send_dependents_are_unconfirmed_if_previous_block_is_unconfirmed() {
        let ctx = LedgerContext::empty();
        let mut txn = ctx.ledger.rw_txn();

        let mut send1 = ctx.genesis_block_factory().send(&txn).build();
        ctx.ledger.process(&mut txn, &mut send1).unwrap();

        let mut send2 = ctx.genesis_block_factory().send(&txn).build();
        ctx.ledger.process(&mut txn, &mut send2).unwrap();

        assert_eq!(
            ctx.ledger
                .dependents_confirmed_for_unsaved_block(&txn, &send2),
            false
        );
    }

    #[test]
    fn open_dependents_are_unconfirmed_if_send_block_is_unconfirmed() {
        let ctx = LedgerContext::empty();
        let mut txn = ctx.ledger.rw_txn();
        let destination = ctx.block_factory();

        let mut send = ctx
            .genesis_block_factory()
            .send(&txn)
            .link(destination.account())
            .build();
        ctx.ledger.process(&mut txn, &mut send).unwrap();

        let mut open = destination.open(&txn, send.hash()).build();
        ctx.ledger.process(&mut txn, &mut open).unwrap();

        assert_eq!(
            ctx.ledger
                .dependents_confirmed_for_unsaved_block(&txn, &open),
            false
        );
    }

    #[test]
    fn open_dependents_are_confirmed_if_send_block_is_confirmed() {
        let ctx = LedgerContext::empty();
        let mut txn = ctx.ledger.rw_txn();
        let destination = ctx.block_factory();

        let mut send = ctx
            .genesis_block_factory()
            .send(&txn)
            .link(destination.account())
            .build();
        ctx.ledger.process(&mut txn, &mut send).unwrap();
        ctx.ledger.confirm(&mut txn, send.hash());

        let mut open = destination.open(&txn, send.hash()).build();
        ctx.ledger.process(&mut txn, &mut open).unwrap();

        assert_eq!(
            ctx.ledger
                .dependents_confirmed_for_unsaved_block(&txn, &open),
            true
        );
    }

    #[test]
    fn receive_dependents_are_unconfirmed_if_send_block_is_unconfirmed() {
        let ctx = LedgerContext::empty();
        let mut txn = ctx.ledger.rw_txn();

        let destination = ctx.block_factory();

        let mut send1 = ctx
            .genesis_block_factory()
            .send(&txn)
            .link(destination.account())
            .build();
        ctx.ledger.process(&mut txn, &mut send1).unwrap();
        ctx.ledger.confirm(&mut txn, send1.hash());

        let mut send2 = ctx
            .genesis_block_factory()
            .send(&txn)
            .link(destination.account())
            .build();
        ctx.ledger.process(&mut txn, &mut send2).unwrap();

        let mut open = destination.open(&txn, send1.hash()).build();
        ctx.ledger.process(&mut txn, &mut open).unwrap();
        ctx.ledger.confirm(&mut txn, open.hash());

        let mut receive = destination.receive(&txn, send2.hash()).build();
        ctx.ledger.process(&mut txn, &mut receive).unwrap();

        assert_eq!(
            ctx.ledger
                .dependents_confirmed_for_unsaved_block(&txn, &receive),
            false
        );
    }

    #[test]
    fn receive_dependents_are_unconfirmed_if_previous_block_is_unconfirmed() {
        let ctx = LedgerContext::empty();
        let mut txn = ctx.ledger.rw_txn();

        let destination = ctx.block_factory();

        let mut send1 = ctx
            .genesis_block_factory()
            .send(&txn)
            .link(destination.account())
            .build();
        ctx.ledger.process(&mut txn, &mut send1).unwrap();
        ctx.ledger.confirm(&mut txn, send1.hash());

        let mut send2 = ctx
            .genesis_block_factory()
            .send(&txn)
            .link(destination.account())
            .build();
        ctx.ledger.process(&mut txn, &mut send2).unwrap();
        ctx.ledger.confirm(&mut txn, send2.hash());

        let mut open = destination.open(&txn, send1.hash()).build();
        ctx.ledger.process(&mut txn, &mut open).unwrap();

        let mut receive = destination.receive(&txn, send2.hash()).build();
        ctx.ledger.process(&mut txn, &mut receive).unwrap();

        assert_eq!(
            ctx.ledger
                .dependents_confirmed_for_unsaved_block(&txn, &receive),
            false
        );
    }

    #[test]
    fn receive_dependents_are_confirmed_if_previous_block_and_send_block_are_confirmed() {
        let ctx = LedgerContext::empty();
        let mut txn = ctx.ledger.rw_txn();

        let destination = ctx.block_factory();

        let mut send1 = ctx
            .genesis_block_factory()
            .send(&txn)
            .link(destination.account())
            .build();
        ctx.ledger.process(&mut txn, &mut send1).unwrap();
        ctx.ledger.confirm(&mut txn, send1.hash());

        let mut send2 = ctx
            .genesis_block_factory()
            .send(&txn)
            .link(destination.account())
            .build();
        ctx.ledger.process(&mut txn, &mut send2).unwrap();
        ctx.ledger.confirm(&mut txn, send2.hash());

        let mut open = destination.open(&txn, send1.hash()).build();
        ctx.ledger.process(&mut txn, &mut open).unwrap();
        ctx.ledger.confirm(&mut txn, open.hash());

        let mut receive = destination.receive(&txn, send2.hash()).build();
        ctx.ledger.process(&mut txn, &mut receive).unwrap();

        assert_eq!(
            ctx.ledger
                .dependents_confirmed_for_unsaved_block(&txn, &receive),
            true
        );
    }

    #[test]
    fn dependents_confirmed_pruning() {
        let ctx = LedgerContext::empty();
        let mut txn = ctx.ledger.rw_txn();
        ctx.ledger.enable_pruning();
        let destination = ctx.block_factory();

        let mut send1 = ctx
            .genesis_block_factory()
            .send(&txn)
            .amount_sent(Amount::raw(1))
            .link(destination.account())
            .build();
        ctx.ledger.process(&mut txn, &mut send1).unwrap();
        ctx.ledger.confirm(&mut txn, send1.hash());

        let mut send2 = ctx
            .genesis_block_factory()
            .send(&txn)
            .link(destination.account())
            .build();
        ctx.ledger.process(&mut txn, &mut send2).unwrap();
        ctx.ledger.confirm(&mut txn, send2.hash());

        assert_eq!(ctx.ledger.pruning_action(&mut txn, &send2.hash(), 1), 2);

        let receive1 = BlockBuilder::state()
            .account(destination.account())
            .previous(0)
            .balance(Amount::raw(1))
            .link(send1.hash())
            .sign(&destination.key)
            .build();
        assert_eq!(
            ctx.ledger
                .dependents_confirmed_for_unsaved_block(&txn, &receive1),
            true
        );
    }
}

#[test]
fn block_confirmed() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();
    assert_eq!(
        ctx.ledger
            .confirmed()
            .block_exists_or_pruned(&txn, &DEV_GENESIS_HASH),
        true
    );

    let destination = ctx.block_factory();
    let mut send = ctx
        .genesis_block_factory()
        .send(&txn)
        .link(destination.account())
        .build();

    // Must be safe against non-existing blocks
    assert_eq!(
        ctx.ledger
            .confirmed()
            .block_exists_or_pruned(&txn, &send.hash()),
        false
    );

    ctx.ledger.process(&mut txn, &mut send).unwrap();
    assert_eq!(
        ctx.ledger
            .confirmed()
            .block_exists_or_pruned(&txn, &send.hash()),
        false
    );

    ctx.ledger.confirm(&mut txn, send.hash());
    assert_eq!(
        ctx.ledger
            .confirmed()
            .block_exists_or_pruned(&txn, &send.hash()),
        true
    );
}

#[test]
fn ledger_cache() {
    let ctx = LedgerContext::empty();
    let genesis = ctx.genesis_block_factory();
    let total = 10u64;

    struct ExpectedCache {
        account_count: u64,
        block_count: u64,
        cemented_count: u64,
        pruned_count: u64,
    }

    // Check existing ledger (incremental cache update) and reload on a new ledger
    for i in 0..total {
        let mut expected = ExpectedCache {
            account_count: 1 + i,
            block_count: 1 + 2 * (i + 1) - 2,
            cemented_count: 1 + 2 * (i + 1) - 2,
            pruned_count: i,
        };

        let check_impl = |ledger: &Ledger, expected: &ExpectedCache| {
            assert_eq!(ledger.account_count(), expected.account_count);
            assert_eq!(ledger.block_count(), expected.block_count);
            assert_eq!(ledger.cemented_count(), expected.cemented_count);
            assert_eq!(ledger.pruned_count(), expected.pruned_count);
        };

        let cache_check = |ledger: &Ledger, expected: &ExpectedCache| {
            check_impl(ledger, expected);

            ctx.ledger.store.cache.reset();
            let new_ledger = Ledger::new(
                ctx.ledger.store.clone(),
                LEDGER_CONSTANTS_STUB.clone(),
                Amount::zero(),
                Arc::new(RepWeightCache::new()),
            )
            .unwrap();
            check_impl(&new_ledger, expected);
        };

        let destination = ctx.block_factory();
        let send = {
            let mut txn = ctx.ledger.rw_txn();
            let mut send = genesis.send(&txn).link(destination.account()).build();
            ctx.ledger.process(&mut txn, &mut send).unwrap();
            expected.block_count += 1;
            send
        };
        cache_check(&ctx.ledger, &expected);

        let open = {
            let mut txn = ctx.ledger.rw_txn();
            let mut open = destination.open(&txn, send.hash()).build();
            ctx.ledger.process(&mut txn, &mut open).unwrap();
            expected.block_count += 1;
            expected.account_count += 1;
            open
        };
        cache_check(&ctx.ledger, &expected);

        {
            let mut txn = ctx.ledger.rw_txn();
            ctx.inc_confirmation_height(&mut txn, &DEV_GENESIS_ACCOUNT);
            ctx.ledger
                .store
                .cache
                .cemented_count
                .fetch_add(1, Ordering::Relaxed);
            expected.cemented_count += 1;
        }
        cache_check(&ctx.ledger, &expected);

        {
            let mut txn = ctx.ledger.rw_txn();
            ctx.inc_confirmation_height(&mut txn, &destination.account());
            ctx.ledger
                .store
                .cache
                .cemented_count
                .fetch_add(1, Ordering::Relaxed);
            expected.cemented_count += 1;
        }
        cache_check(&ctx.ledger, &expected);

        {
            let mut txn = ctx.ledger.rw_txn();
            ctx.ledger.store.pruned.put(&mut txn, &open.hash());
            ctx.ledger
                .store
                .cache
                .pruned_count
                .fetch_add(1, Ordering::Relaxed);
            expected.pruned_count += 1;
        }
        cache_check(&ctx.ledger, &expected);
    }
}

#[test]
fn is_send_genesis() {
    assert_eq!(DEV_GENESIS_BLOCK.is_send(), false);
}

#[test]
fn is_send_state() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();
    let open = setup_open_block(&ctx, &mut txn);
    assert_eq!(open.send_block.is_send(), true);
    assert_eq!(open.open_block.is_send(), false);
}

#[test]
fn is_send_legacy() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();
    let open = setup_legacy_open_block(&ctx, &mut txn);
    assert_eq!(open.send_block.is_send(), true);
    assert_eq!(open.open_block.is_send(), false);
}

#[test]
fn sideband_height() {
    let ctx = LedgerContext::empty();
    let mut txn = ctx.ledger.rw_txn();
    let genesis = ctx.genesis_block_factory();
    let dest1 = ctx.block_factory();
    let dest2 = ctx.block_factory();
    let dest3 = ctx.block_factory();

    let mut send = genesis
        .legacy_send(&txn)
        .destination(genesis.account())
        .build();
    ctx.ledger.process(&mut txn, &mut send).unwrap();

    let mut receive = genesis.legacy_receive(&txn, send.hash()).build();
    ctx.ledger.process(&mut txn, &mut receive).unwrap();

    let mut change = genesis.legacy_change(&txn).build();
    ctx.ledger.process(&mut txn, &mut change).unwrap();

    let mut state_send1 = genesis.send(&txn).link(dest1.account()).build();
    ctx.ledger.process(&mut txn, &mut state_send1).unwrap();

    let mut state_send2 = genesis.send(&txn).link(dest2.account()).build();
    ctx.ledger.process(&mut txn, &mut state_send2).unwrap();

    let mut state_send3 = genesis.send(&txn).link(dest3.account()).build();
    ctx.ledger.process(&mut txn, &mut state_send3).unwrap();

    let mut state_open = dest1.open(&txn, state_send1.hash()).build();
    ctx.ledger.process(&mut txn, &mut state_open).unwrap();

    let mut epoch = dest1.epoch_v1(&txn).build();
    ctx.ledger.process(&mut txn, &mut epoch).unwrap();

    let mut epoch_open = dest2.epoch_v1_open().build();
    ctx.ledger.process(&mut txn, &mut epoch_open).unwrap();

    let mut state_receive = dest2.receive(&txn, state_send2.hash()).build();
    ctx.ledger.process(&mut txn, &mut state_receive).unwrap();

    let mut open = dest3.legacy_open(state_send3.hash()).build();
    ctx.ledger.process(&mut txn, &mut open).unwrap();

    let assert_sideband_height = |hash: &BlockHash, expected_height: u64| {
        let block = ctx.ledger.any().get_block(&txn, hash).unwrap();
        assert_eq!(block.height(), expected_height);
    };

    assert_sideband_height(&DEV_GENESIS_HASH, 1);
    assert_sideband_height(&send.hash(), 2);
    assert_sideband_height(&receive.hash(), 3);
    assert_sideband_height(&change.hash(), 4);
    assert_sideband_height(&state_send1.hash(), 5);
    assert_sideband_height(&state_send2.hash(), 6);
    assert_sideband_height(&state_send3.hash(), 7);

    assert_sideband_height(&state_open.hash(), 1);
    assert_sideband_height(&epoch.hash(), 2);

    assert_sideband_height(&epoch_open.hash(), 1);
    assert_sideband_height(&state_receive.hash(), 2);

    assert_sideband_height(&open.hash(), 1);
}

#[test]
fn configured_peers_response() {
    let endpoint = TEST_ENDPOINT_1;
    let now = new_test_timestamp();
    let ledger = Ledger::new_null_builder().peers([(endpoint, now)]).finish();
    let tx = ledger.read_txn();
    assert_eq!(ledger.store.peer.iter(&tx).next().unwrap(), (endpoint, now));
}
