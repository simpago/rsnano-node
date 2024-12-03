use rsnano_core::{Block, BlockHash};
use rsnano_store_lmdb::{LmdbStore, Transaction};

/// Goes back in the block history until it finds a block with representative information
pub struct RepresentativeBlockFinder<'a> {
    txn: &'a dyn Transaction,
    store: &'a LmdbStore,
}

impl<'a> RepresentativeBlockFinder<'a> {
    pub fn new(txn: &'a dyn Transaction, store: &'a LmdbStore) -> Self {
        Self { txn, store }
    }

    pub fn find_rep_block(&self, hash: BlockHash) -> BlockHash {
        let mut current = hash;
        let mut result = BlockHash::zero();
        while result.is_zero() {
            let Some(block) = self.store.block.get(self.txn, &current) else {
                return BlockHash::zero();
            };
            (current, result) = match &*block {
                Block::LegacySend(_) => (block.previous(), BlockHash::zero()),
                Block::LegacyReceive(_) => (block.previous(), BlockHash::zero()),
                Block::LegacyOpen(_) => (BlockHash::zero(), block.hash()),
                Block::LegacyChange(_) => (BlockHash::zero(), block.hash()),
                Block::State(_) => (BlockHash::zero(), block.hash()),
            };
        }

        result
    }
}
