use crate::{
    core::{
        Block, BlockHash, BlockVisitor, ChangeBlock, OpenBlock, ReceiveBlock, SendBlock, StateBlock,
    },
    ledger::{datastore::Transaction, Ledger, LedgerConstants},
};

pub(crate) struct DependentBlockVisitor<'a> {
    ledger: &'a Ledger,
    constants: &'a LedgerConstants,
    transaction: &'a dyn Transaction,
    pub result: [BlockHash; 2],
}

impl<'a> DependentBlockVisitor<'a> {
    pub(crate) fn new(
        ledger: &'a Ledger,
        constants: &'a LedgerConstants,
        transaction: &'a dyn Transaction,
    ) -> Self {
        Self {
            ledger,
            constants,
            transaction,
            result: [BlockHash::zero(); 2],
        }
    }
}

impl<'a> BlockVisitor for DependentBlockVisitor<'a> {
    fn send_block(&mut self, block: &SendBlock) {
        self.result[0] = block.hashables.previous
    }

    fn receive_block(&mut self, block: &ReceiveBlock) {
        self.result[0] = block.previous();
        self.result[1] = block.source();
    }

    fn open_block(&mut self, block: &OpenBlock) {
        if block.hashables.source
            != self
                .constants
                .genesis
                .read()
                .unwrap()
                .as_block()
                .account()
                .into()
        {
            self.result[0] = block.source();
        }
    }

    fn change_block(&mut self, block: &ChangeBlock) {
        self.result[0] = block.previous();
    }

    fn state_block(&mut self, block: &StateBlock) {
        self.result[0] = block.previous();
        self.result[1] = block.link().into();
        // ledger.is_send will check the sideband first, if block_a has a loaded sideband the check that previous block exists can be skipped
        if self.ledger.is_epoch_link(&block.link())
            || ((block.sideband().is_some()
                || self
                    .ledger
                    .store
                    .block()
                    .exists(self.transaction, &block.previous()))
                && self.ledger.is_send(self.transaction, block))
        {
            self.result[1] = BlockHash::zero()
        }
    }
}
