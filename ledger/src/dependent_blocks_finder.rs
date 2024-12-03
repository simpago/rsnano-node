use crate::ledger::Ledger;
use rsnano_core::{Block, BlockBase, BlockHash, DependentBlocks, SavedBlock, StateBlock};
use rsnano_store_lmdb::Transaction;

/// Finds all dependent blocks for a given block.
/// There can be at most two dependencies per block, namely "previous" and "link/source".
pub struct DependentBlocksFinder<'a> {
    ledger: &'a Ledger,
    txn: &'a dyn Transaction,
}

impl<'a> DependentBlocksFinder<'a> {
    pub fn new(ledger: &'a Ledger, txn: &'a dyn Transaction) -> Self {
        Self { ledger, txn }
    }

    pub fn find_dependent_blocks(&self, block: &SavedBlock) -> DependentBlocks {
        block.dependent_blocks(
            &self.ledger.constants.epochs,
            &self.ledger.constants.genesis_account,
        )
    }

    pub fn find_dependent_blocks_for_unsaved_block(&self, block: &Block) -> DependentBlocks {
        match block {
            Block::LegacySend(b) => b.dependent_blocks(),
            Block::LegacyChange(b) => b.dependent_blocks(),
            Block::LegacyReceive(b) => b.dependent_blocks(),
            Block::LegacyOpen(b) => b.dependent_blocks(&self.ledger.constants.genesis_account),
            // a ledger lookup is needed if it is a state block!
            Block::State(state) => {
                let linked_block = if self.is_receive_or_change(state) {
                    state.link().into()
                } else {
                    BlockHash::zero()
                };
                DependentBlocks::new(block.previous(), linked_block)
            }
        }
    }

    fn is_receive_or_change(&self, state: &StateBlock) -> bool {
        !self.ledger.is_epoch_link(&state.link()) && !self.is_send(state)
    }

    // This function is used in place of block.is_send() as it is tolerant to the block not having the sideband information loaded
    // This is needed for instance in vote generation on forks which have not yet had sideband information attached
    fn is_send(&self, block: &StateBlock) -> bool {
        if block.previous().is_zero() {
            return false;
        }

        let previous_balance = self
            .ledger
            .any()
            .block_balance(self.txn, &block.previous())
            .unwrap_or_default();

        block.balance() < previous_balance
    }
}
