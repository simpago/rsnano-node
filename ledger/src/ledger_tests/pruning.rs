use crate::ledger_tests::helpers::upgrade_genesis_to_epoch_v1;
use crate::ledger_tests::LedgerContext;
use crate::{ledger_constants::LEDGER_CONSTANTS_STUB, DEV_GENESIS_HASH};
use rsnano_core::{
    work::{WorkPool, STUB_WORK_POOL},
    Amount, Epoch, PendingKey, TestBlockBuilder,
};

#[test]
fn pruning_action() {
    let ctx = LedgerContext::empty();
    ctx.ledger.enable_pruning();
    let mut txn = ctx.ledger.rw_txn();
    let genesis = ctx.genesis_block_factory();

    let mut send1 = genesis
        .send(&txn)
        .amount_sent(100)
        .link(genesis.account())
        .build();
    ctx.ledger.process(&mut txn, &mut send1).unwrap();
    ctx.ledger.confirm(&mut txn, send1.hash());

    let mut send2 = genesis
        .send(&txn)
        .amount_sent(100)
        .link(genesis.account())
        .build();
    ctx.ledger.process(&mut txn, &mut send2).unwrap();
    ctx.ledger.confirm(&mut txn, send2.hash());

    // Prune...
    assert_eq!(ctx.ledger.pruning_action(&mut txn, &send1.hash(), 1), 1);
    assert_eq!(ctx.ledger.pruning_action(&mut txn, &DEV_GENESIS_HASH, 1), 0);
    assert!(ctx
        .ledger
        .any()
        .get_pending(&txn, &PendingKey::new(genesis.account(), send1.hash()))
        .is_some());

    assert_eq!(ctx.ledger.store.block.exists(&txn, &send1.hash()), false);

    assert!(ctx.ledger.any().block_exists_or_pruned(&txn, &send1.hash()),);

    assert!(ctx.ledger.store.pruned.exists(&txn, &send1.hash()),);

    assert!(ctx.ledger.store.block.exists(&txn, &DEV_GENESIS_HASH));
    assert!(ctx.ledger.store.block.exists(&txn, &send2.hash()));

    // Receiving pruned block
    let mut receive1 = TestBlockBuilder::state()
        .account(genesis.account())
        .previous(send2.hash())
        .balance(LEDGER_CONSTANTS_STUB.genesis_amount - Amount::raw(100))
        .link(send1.hash())
        .key(&genesis.key)
        .work(STUB_WORK_POOL.generate_dev2(send2.hash().into()).unwrap())
        .build();
    ctx.ledger.process(&mut txn, &mut receive1).unwrap();

    assert!(ctx.ledger.store.block.exists(&txn, &receive1.hash()));
    assert_eq!(
        ctx.ledger
            .any()
            .get_pending(&txn, &PendingKey::new(genesis.account(), send1.hash())),
        None
    );
    let receive1_stored = ctx.ledger.any().get_block(&txn, &receive1.hash()).unwrap();
    assert_eq!(&receive1, &*receive1_stored);
    assert_eq!(receive1_stored.height(), 4);
    assert!(receive1_stored.is_receive());

    // Middle block pruning
    assert!(ctx.ledger.store.block.exists(&txn, &send2.hash()));
    ctx.ledger.confirm(&mut txn, send2.hash());
    assert_eq!(ctx.ledger.pruning_action(&mut txn, &send2.hash(), 1), 1);
    assert!(ctx.ledger.store.pruned.exists(&txn, &send2.hash()));
    assert_eq!(ctx.ledger.store.block.exists(&txn, &send2.hash()), false);
    assert_eq!(
        ctx.ledger.store.account.count(&txn),
        ctx.ledger.account_count()
    );
    assert_eq!(
        ctx.ledger.store.pruned.count(&txn),
        ctx.ledger.pruned_count()
    );
    assert_eq!(
        ctx.ledger.store.block.count(&txn),
        ctx.ledger.block_count() - ctx.ledger.pruned_count()
    );
}
#[test]
fn pruning_large_chain() {
    let ctx = LedgerContext::empty();
    ctx.ledger.enable_pruning();
    let genesis = ctx.genesis_block_factory();
    let mut txn = ctx.ledger.rw_txn();
    let send_receive_pairs = 20;
    let mut last_hash = *DEV_GENESIS_HASH;

    for _ in 0..send_receive_pairs {
        let mut send = genesis.send(&txn).link(genesis.account()).build();
        ctx.ledger.process(&mut txn, &mut send).unwrap();

        let mut receive = genesis.receive(&txn, send.hash()).build();
        ctx.ledger.process(&mut txn, &mut receive).unwrap();

        last_hash = receive.hash();
    }
    assert_eq!(
        ctx.ledger.store.block.count(&txn),
        send_receive_pairs * 2 + 1
    );
    ctx.ledger.confirm(&mut txn, last_hash);

    // Pruning action
    assert_eq!(
        ctx.ledger.pruning_action(&mut txn, &last_hash, 5),
        send_receive_pairs * 2
    );

    assert!(ctx.ledger.store.pruned.exists(&txn, &last_hash));
    assert!(ctx.ledger.store.block.exists(&txn, &DEV_GENESIS_HASH));
    assert_eq!(ctx.ledger.store.block.exists(&txn, &last_hash), false);
    assert_eq!(
        ctx.ledger.store.pruned.count(&txn),
        ctx.ledger.pruned_count()
    );
    assert_eq!(
        ctx.ledger.store.block.count(&txn),
        ctx.ledger.block_count() - ctx.ledger.pruned_count()
    );
    assert_eq!(ctx.ledger.store.pruned.count(&txn), send_receive_pairs * 2);
    assert_eq!(ctx.ledger.store.block.count(&txn), 1);
}

#[test]
fn pruning_source_rollback() {
    let ctx = LedgerContext::empty();
    ctx.ledger.enable_pruning();
    let genesis = ctx.genesis_block_factory();
    let mut txn = ctx.ledger.rw_txn();

    upgrade_genesis_to_epoch_v1(&ctx, &mut txn);

    let mut send1 = genesis
        .send(&txn)
        .amount_sent(100)
        .link(genesis.account())
        .build();
    ctx.ledger.process(&mut txn, &mut send1).unwrap();

    let mut send2 = genesis
        .send(&txn)
        .amount_sent(100)
        .link(genesis.account())
        .build();
    ctx.ledger.process(&mut txn, &mut send2).unwrap();
    ctx.ledger.confirm(&mut txn, send2.hash());

    // Pruning action
    assert_eq!(ctx.ledger.pruning_action(&mut txn, &send1.hash(), 1), 2);

    // Receiving pruned block
    let mut receive1 = TestBlockBuilder::state()
        .account(genesis.account())
        .previous(send2.hash())
        .balance(LEDGER_CONSTANTS_STUB.genesis_amount - Amount::raw(100))
        .link(send1.hash())
        .key(&genesis.key)
        .work(STUB_WORK_POOL.generate_dev2(send2.hash().into()).unwrap())
        .build();
    ctx.ledger.process(&mut txn, &mut receive1).unwrap();

    // Rollback receive block
    ctx.ledger.rollback(&mut txn, &receive1.hash()).unwrap();
    let info2 = ctx
        .ledger
        .any()
        .get_pending(&txn, &PendingKey::new(genesis.account(), send1.hash()))
        .unwrap();
    assert_ne!(info2.source, genesis.account()); // Tradeoff to not store pruned blocks accounts
    assert_eq!(info2.amount, Amount::raw(100));
    assert_eq!(info2.epoch, Epoch::Epoch1);

    // Process receive block again
    ctx.ledger.process(&mut txn, &mut receive1).unwrap();

    assert_eq!(
        ctx.ledger
            .any()
            .get_pending(&txn, &PendingKey::new(genesis.account(), send1.hash())),
        None
    );
    assert_eq!(ctx.ledger.pruned_count(), 2);
    assert_eq!(ctx.ledger.block_count(), 5);
}

#[test]
fn pruning_source_rollback_legacy() {
    let ctx = LedgerContext::empty();
    ctx.ledger.enable_pruning();
    let genesis = ctx.genesis_block_factory();
    let mut txn = ctx.ledger.rw_txn();

    let mut send1 = genesis
        .legacy_send(&txn)
        .destination(genesis.account())
        .amount(100)
        .build();
    ctx.ledger.process(&mut txn, &mut send1).unwrap();

    let destination = ctx.block_factory();
    let mut send2 = genesis
        .legacy_send(&txn)
        .destination(destination.account())
        .amount(100)
        .build();
    ctx.ledger.process(&mut txn, &mut send2).unwrap();

    let mut send3 = genesis
        .legacy_send(&txn)
        .destination(genesis.account())
        .amount(100)
        .build();
    ctx.ledger.process(&mut txn, &mut send3).unwrap();
    ctx.ledger.confirm(&mut txn, send2.hash());

    // Pruning action
    assert_eq!(ctx.ledger.pruning_action(&mut txn, &send2.hash(), 1), 2);

    // Receiving pruned block
    let mut receive1 = TestBlockBuilder::legacy_receive()
        .previous(send3.hash())
        .source(send1.hash())
        .sign(&genesis.key)
        .work(STUB_WORK_POOL.generate_dev2(send3.hash().into()).unwrap())
        .build();
    ctx.ledger.process(&mut txn, &mut receive1).unwrap();

    // Rollback receive block
    ctx.ledger.rollback(&mut txn, &receive1.hash()).unwrap();

    let info3 = ctx
        .ledger
        .any()
        .get_pending(&txn, &PendingKey::new(genesis.account(), send1.hash()))
        .unwrap();
    assert_ne!(info3.source, genesis.account()); // Tradeoff to not store pruned blocks accounts
    assert_eq!(info3.amount, Amount::raw(100));
    assert_eq!(info3.epoch, Epoch::Epoch0);

    // Process receive block again
    ctx.ledger.process(&mut txn, &mut receive1).unwrap();

    assert_eq!(
        ctx.ledger
            .any()
            .get_pending(&txn, &PendingKey::new(genesis.account(), send1.hash())),
        None
    );
    assert_eq!(ctx.ledger.pruned_count(), 2);
    assert_eq!(ctx.ledger.block_count(), 5);

    // Receiving pruned block (open)
    let mut open1 = TestBlockBuilder::legacy_open()
        .source(send2.hash())
        .sign(&destination.key)
        .work(
            STUB_WORK_POOL
                .generate_dev2(destination.account().into())
                .unwrap(),
        )
        .build();
    ctx.ledger.process(&mut txn, &mut open1).unwrap();

    // Rollback open block
    ctx.ledger.rollback(&mut txn, &open1.hash()).unwrap();

    let info4 = ctx
        .ledger
        .any()
        .get_pending(&txn, &PendingKey::new(destination.account(), send2.hash()))
        .unwrap();
    assert_ne!(info4.source, genesis.account()); // Tradeoff to not store pruned blocks accounts
    assert_eq!(info4.amount, Amount::raw(100));
    assert_eq!(info4.epoch, Epoch::Epoch0);

    // Process open block again
    ctx.ledger.process(&mut txn, &mut open1).unwrap();
    assert_eq!(
        ctx.ledger
            .any()
            .get_pending(&txn, &PendingKey::new(destination.account(), send2.hash())),
        None
    );
    assert_eq!(ctx.ledger.pruned_count(), 2);
    assert_eq!(ctx.ledger.block_count(), 6);
}

#[test]
fn pruning_legacy_blocks() {
    let ctx = LedgerContext::empty();
    ctx.ledger.enable_pruning();
    let mut txn = ctx.ledger.rw_txn();
    let genesis = ctx.genesis_block_factory();
    let destination = ctx.block_factory();

    let mut send1 = genesis
        .legacy_send(&txn)
        .destination(genesis.account())
        .build();
    ctx.ledger.process(&mut txn, &mut send1).unwrap();

    let mut receive1 = genesis.legacy_receive(&txn, send1.hash()).build();
    ctx.ledger.process(&mut txn, &mut receive1).unwrap();

    let mut change1 = genesis
        .legacy_change(&txn)
        .representative(destination.public_key())
        .build();
    ctx.ledger.process(&mut txn, &mut change1).unwrap();

    let mut send2 = genesis
        .legacy_send(&txn)
        .destination(destination.account())
        .build();
    ctx.ledger.process(&mut txn, &mut send2).unwrap();

    let mut open1 = destination.legacy_open(send2.hash()).build();
    ctx.ledger.process(&mut txn, &mut open1).unwrap();

    let mut send3 = destination
        .legacy_send(&txn)
        .destination(genesis.account())
        .build();
    ctx.ledger.process(&mut txn, &mut send3).unwrap();
    ctx.ledger.confirm(&mut txn, change1.hash());
    ctx.ledger.confirm(&mut txn, open1.hash());

    // Pruning action
    assert_eq!(ctx.ledger.pruning_action(&mut txn, &change1.hash(), 2), 3);

    assert_eq!(ctx.ledger.pruning_action(&mut txn, &open1.hash(), 1), 1);

    assert!(ctx.ledger.store.block.exists(&txn, &DEV_GENESIS_HASH));
    assert_eq!(ctx.ledger.store.block.exists(&txn, &send1.hash()), false);
    assert_eq!(ctx.ledger.store.pruned.exists(&txn, &send1.hash()), true);
    assert_eq!(ctx.ledger.store.block.exists(&txn, &receive1.hash()), false);
    assert_eq!(ctx.ledger.store.pruned.exists(&txn, &receive1.hash()), true);
    assert_eq!(ctx.ledger.store.block.exists(&txn, &change1.hash()), false);
    assert_eq!(ctx.ledger.store.pruned.exists(&txn, &change1.hash()), true);
    assert_eq!(ctx.ledger.store.block.exists(&txn, &send2.hash()), true);
    assert_eq!(ctx.ledger.store.block.exists(&txn, &open1.hash()), false);
    assert_eq!(ctx.ledger.store.pruned.exists(&txn, &open1.hash()), true);
    assert_eq!(ctx.ledger.store.block.exists(&txn, &send3.hash()), true);
    assert_eq!(ctx.ledger.pruned_count(), 4);
    assert_eq!(ctx.ledger.block_count(), 7);
    assert_eq!(ctx.ledger.store.pruned.count(&txn), 4);
    assert_eq!(ctx.ledger.store.block.count(&txn), 3);
}

#[test]
fn pruning_safe_functions() {
    let ctx = LedgerContext::empty();
    ctx.ledger.enable_pruning();
    let mut txn = ctx.ledger.rw_txn();
    let genesis = ctx.genesis_block_factory();

    let mut send1 = genesis.send(&txn).link(genesis.account()).build();
    ctx.ledger.process(&mut txn, &mut send1).unwrap();

    let mut send2 = genesis.send(&txn).link(genesis.account()).build();
    ctx.ledger.process(&mut txn, &mut send2).unwrap();
    ctx.ledger.confirm(&mut txn, send1.hash());

    // Pruning action
    assert_eq!(ctx.ledger.pruning_action(&mut txn, &send1.hash(), 1), 1);

    // Safe ledger actions
    assert!(ctx
        .ledger
        .any()
        .block_balance(&txn, &send1.hash())
        .is_none());
    assert_eq!(
        ctx.ledger.any().block_balance(&txn, &send2.hash()).unwrap(),
        send2.balance_field().unwrap()
    );

    assert_eq!(ctx.ledger.any().block_amount(&txn, &send2.hash()), None);
    assert_eq!(ctx.ledger.any().block_account(&txn, &send1.hash()), None);
    assert_eq!(
        ctx.ledger.any().block_account(&txn, &send2.hash()),
        Some(genesis.account())
    );
}

#[test]
fn hash_root_random() {
    let ctx = LedgerContext::empty();
    ctx.ledger.enable_pruning();
    let mut txn = ctx.ledger.rw_txn();
    let genesis = ctx.genesis_block_factory();

    let mut send1 = genesis.send(&txn).link(genesis.account()).build();
    ctx.ledger.process(&mut txn, &mut send1).unwrap();

    let mut send2 = genesis.send(&txn).link(genesis.account()).build();
    ctx.ledger.process(&mut txn, &mut send2).unwrap();
    ctx.ledger.confirm(&mut txn, send1.hash());

    // Pruning action
    assert_eq!(ctx.ledger.pruning_action(&mut txn, &send1.hash(), 1), 1);

    // Prunned block will not be included in the random selection because it's not in the blocks set
    {
        let mut done = false;
        let mut iteration = 0;
        while !done && iteration < 42 {
            iteration += 1;
            let blocks = ctx.ledger.random_blocks(&txn, 10);
            // Random blocks should repeat if the ledger is smaller than the requested count
            assert_eq!(blocks.len(), 10);
            let first = &blocks[0];
            done = first.hash() == send1.hash();
        }
        assert_eq!(done, false);
    }

    // Genesis and send2 should be included in the random selection
    {
        let mut done = false;
        let mut iteration = 0;
        while !done {
            iteration += 1;
            let blocks = ctx.ledger.random_blocks(&txn, 1);
            assert_eq!(blocks.len(), 1);
            let first = &blocks[0];
            done = first.hash() == send2.hash();
            assert!(iteration < 1000);
        }
    }
    {
        let mut done = false;
        let mut iteration = 0;
        while !done {
            iteration += 1;
            let blocks = ctx.ledger.random_blocks(&txn, 1);
            let first = &blocks[0];
            done = first.hash() == *DEV_GENESIS_HASH;
            assert!(iteration < 1000);
        }
    }
}
