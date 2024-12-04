use rsnano_core::{
    Amount, Block, PrivateKey, StateBlock, UncheckedInfo, UncheckedKey, DEV_GENESIS_KEY,
};
use rsnano_ledger::{DEV_GENESIS_ACCOUNT, DEV_GENESIS_HASH, DEV_GENESIS_PUB_KEY};
use rsnano_node::{block_processing::UncheckedMap, stats::Stats};
use std::{sync::Arc, time::Duration};
use test_helpers::{assert_timely, assert_timely_eq};

#[test]
fn one_bootstrap() {
    let unchecked = UncheckedMap::new(65536, Arc::new(Stats::default()), false);
    let block1 = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::raw(1),
        (*DEV_GENESIS_ACCOUNT).into(),
        &DEV_GENESIS_KEY,
        0,
    ));
    unchecked.put(block1.hash().into(), UncheckedInfo::new(block1.clone()));

    // Waits for the block1 to get saved in the database
    assert_timely(Duration::from_secs(10), || {
        unchecked.get(&block1.hash().into()).len() > 0
    });
    let mut dependencies = Vec::new();
    unchecked.for_each(
        |key, _| {
            dependencies.push(key.hash);
        },
        || true,
    );
    let hash1 = dependencies[0];
    assert_eq!(block1.hash(), hash1);
    let mut blocks = unchecked.get(&hash1.into());
    assert_eq!(blocks.len(), 1);
    let block2 = blocks.remove(0).block;
    assert_eq!(block2.hash(), block1.hash());
}

// This test checks for basic operations in the unchecked table such as putting a new block, retrieving it, and
// deleting it from the database
#[test]
fn simple() {
    let unchecked = UncheckedMap::new(65536, Arc::new(Stats::default()), false);
    let block = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::raw(1),
        (*DEV_GENESIS_ACCOUNT).into(),
        &DEV_GENESIS_KEY,
        0,
    ));
    // Asserts the block wasn't added yet to the unchecked table
    let block_listing1 = unchecked.get(&block.previous().into());
    assert!(block_listing1.is_empty());
    // Enqueues a block to be saved on the unchecked table
    unchecked.put(block.previous().into(), UncheckedInfo::new(block.clone()));
    // Waits for the block to get written in the database
    assert_timely(Duration::from_secs(5), || {
        unchecked.get(&block.previous().into()).len() > 0
    });
    // Retrieves the block from the database
    let block_listing2 = unchecked.get(&block.previous().into());
    assert_ne!(block_listing2.len(), 0);
    // Asserts the added block is equal to the retrieved one
    assert_eq!(block_listing2[0].block.hash(), block.hash());
    // Deletes the block from the database
    unchecked.remove(&UncheckedKey::new(block.previous(), block.hash()));
    // Asserts the block is deleted
    let block_listing3 = unchecked.get(&block.previous().into());
    assert!(block_listing3.is_empty());
}

// This test ensures the unchecked table is able to receive more than one block
#[test]
fn multiple() {
    let unchecked = UncheckedMap::new(65536, Arc::new(Stats::default()), false);
    let block = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::raw(1),
        (*DEV_GENESIS_ACCOUNT).into(),
        &DEV_GENESIS_KEY,
        0,
    ));
    // Asserts the block wasn't added yet to the unchecked table
    let block_listing1 = unchecked.get(&block.previous().into());
    assert!(block_listing1.is_empty());

    // Enqueues the first block
    unchecked.put(block.previous().into(), UncheckedInfo::new(block.clone()));
    // Enqueues a second block
    unchecked.put(6.into(), UncheckedInfo::new(block.clone()));
    // Waits for the block to get written in the database
    assert_timely(Duration::from_secs(5), || {
        unchecked.get(&block.previous().into()).len() > 0
    });
    // Waits for and asserts the first block gets saved in the database
    assert_timely(Duration::from_secs(5), || {
        unchecked.get(&6.into()).len() > 0
    });
}

// This test ensures that a block can't occur twice in the unchecked table.
#[test]
fn double_put() {
    let unchecked = UncheckedMap::new(65536, Arc::new(Stats::default()), false);
    let block = Block::State(StateBlock::new(
        *DEV_GENESIS_ACCOUNT,
        *DEV_GENESIS_HASH,
        *DEV_GENESIS_PUB_KEY,
        Amount::MAX - Amount::raw(1),
        (*DEV_GENESIS_ACCOUNT).into(),
        &DEV_GENESIS_KEY,
        0,
    ));
    // Asserts the block wasn't added yet to the unchecked table
    let block_listing1 = unchecked.get(&block.previous().into());
    assert!(block_listing1.is_empty());

    // Enqueues the block to be saved in the unchecked table
    unchecked.put(block.previous().into(), UncheckedInfo::new(block.clone()));
    // Enqueues the block again in an attempt to have it there twice
    unchecked.put(block.previous().into(), UncheckedInfo::new(block.clone()));

    // Waits for and asserts the block was added at least once
    assert_timely(Duration::from_secs(5), || {
        unchecked.get(&block.previous().into()).len() > 0
    });
    // Asserts the block was added at most once -- this is objective of this test.
    let block_listing2 = unchecked.get(&block.previous().into());
    assert_eq!(block_listing2.len(), 1);
}

// Tests that recurrent get calls return the correct values
#[test]
fn multiple_get() {
    let unchecked = UncheckedMap::new(65536, Arc::new(Stats::default()), false);
    // Instantiates three blocks
    let key1 = PrivateKey::new();
    let block1 = Block::State(StateBlock::new(
        key1.account(),
        1.into(),
        *DEV_GENESIS_PUB_KEY,
        Amount::raw(1),
        (*DEV_GENESIS_ACCOUNT).into(),
        &key1,
        0,
    ));
    let key2 = PrivateKey::new();
    let block2 = Block::State(StateBlock::new(
        key2.account(),
        2.into(),
        *DEV_GENESIS_PUB_KEY,
        Amount::raw(1),
        (*DEV_GENESIS_ACCOUNT).into(),
        &key2,
        0,
    ));
    let key3 = PrivateKey::new();
    let block3 = Block::State(StateBlock::new(
        key3.account(),
        3.into(),
        *DEV_GENESIS_PUB_KEY,
        Amount::raw(1),
        (*DEV_GENESIS_ACCOUNT).into(),
        &key3,
        0,
    ));
    // Add the blocks' info to the unchecked table
    unchecked.put(block1.previous().into(), UncheckedInfo::new(block1.clone())); // unchecked1
    unchecked.put(block1.hash().into(), UncheckedInfo::new(block1.clone())); // unchecked2
    unchecked.put(block2.previous().into(), UncheckedInfo::new(block2.clone())); // unchecked3
    unchecked.put(block1.previous().into(), UncheckedInfo::new(block2.clone())); // unchecked1
    unchecked.put(block1.hash().into(), UncheckedInfo::new(block2.clone())); // unchecked2
    unchecked.put(block3.previous().into(), UncheckedInfo::new(block3.clone()));
    unchecked.put(block3.hash().into(), UncheckedInfo::new(block3.clone())); // unchecked4
    unchecked.put(block1.previous().into(), UncheckedInfo::new(block3.clone()));
    // unchecked1

    // count the number of blocks in the unchecked table by counting them one by one
    // we cannot trust the count() method if the backend is rocksdb
    let count_unchecked_blocks_one_by_one = || {
        let mut count = 0;
        unchecked.for_each(
            |_, _| {
                count += 1;
            },
            || true,
        );
        count
    };

    // Waits for the blocks to get saved in the database
    assert_timely_eq(Duration::from_secs(5), count_unchecked_blocks_one_by_one, 8);

    let mut unchecked1 = Vec::new();
    // Asserts the entries will be found for the provided key
    let unchecked1_blocks = unchecked.get(&block1.previous().into());
    assert_eq!(unchecked1_blocks.len(), 3);
    for i in unchecked1_blocks {
        unchecked1.push(i.block.hash());
    }
    // Asserts the payloads where correclty saved
    assert!(unchecked1.contains(&block1.hash()));
    assert!(unchecked1.contains(&block2.hash()));
    assert!(unchecked1.contains(&block3.hash()));
    let mut unchecked2 = Vec::new();
    // Asserts the entries will be found for the provided key
    let unchecked2_blocks = unchecked.get(&block1.hash().into());
    assert_eq!(unchecked2_blocks.len(), 2);
    for i in unchecked2_blocks {
        unchecked2.push(i.block.hash());
    }
    // Asserts the payloads where correctly saved
    assert!(unchecked2.contains(&block1.hash()));
    assert!(unchecked2.contains(&block2.hash()));
    // Asserts the entry is found by the key and the payload is saved
    let unchecked3 = unchecked.get(&block2.previous().into());
    assert_eq!(unchecked3.len(), 1);
    assert_eq!(unchecked3[0].block.hash(), block2.hash());
    // Asserts the entry is found by the key and the payload is saved
    let unchecked4 = unchecked.get(&block3.hash().into());
    assert_eq!(unchecked4.len(), 1);
    assert_eq!(unchecked4[0].block.hash(), block3.hash());
    // Asserts no entry is found for a block that wasn't added
    let unchecked5 = unchecked.get(&block2.hash().into());
    assert_eq!(unchecked5.len(), 0);
}
