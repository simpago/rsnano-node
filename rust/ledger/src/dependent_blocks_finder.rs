use crate::ledger::Ledger;
use rsnano_core::{Block, BlockEnum, BlockHash, DependentBlocks, StateBlock};
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

    pub fn find_dependent_blocks(&self, block: &BlockEnum) -> DependentBlocks {
        if block.sideband().is_none() {
            // a ledger lookup is needed if there is no sideband and it is a state block!
            if let BlockEnum::State(state) = block {
                let linked_block = if self.link_refers_to_block(state) {
                    state.link().into()
                } else {
                    BlockHash::zero()
                };
                return DependentBlocks::new(block.previous(), linked_block);
            }
        }

        block.dependent_blocks(
            &self.ledger.constants.epochs,
            &self.ledger.constants.genesis_account,
        )
    }

    fn link_refers_to_block(&self, state: &StateBlock) -> bool {
        !self.ledger.is_epoch_link(&state.link()) && !self.is_send(state)
    }

    // This function is used in place of block.is_send() as it is tolerant to the block not having the sideband information loaded
    // This is needed for instance in vote generation on forks which have not yet had sideband information attached
    fn is_send(&self, block: &StateBlock) -> bool {
        if block.previous().is_zero() {
            return false;
        }
        if let Some(sideband) = block.sideband() {
            sideband.details.is_send
        } else {
            block.balance()
                < self
                    .ledger
                    .any()
                    .block_balance(self.txn, &block.previous())
                    .unwrap_or_default()
        }
    }
}
