use rsnano_core::{utils::seconds_since_epoch, Account, Block, PendingKey, SavedBlock};
use rsnano_store_lmdb::Transaction;

use crate::Ledger;

use super::BlockValidator;

pub(crate) struct BlockValidatorFactory<'a> {
    ledger: &'a Ledger,
    txn: &'a dyn Transaction,
    block: &'a Block,
}

impl<'a> BlockValidatorFactory<'a> {
    pub(crate) fn new(ledger: &'a Ledger, txn: &'a dyn Transaction, block: &'a Block) -> Self {
        Self { ledger, txn, block }
    }

    pub(crate) fn create_validator(&self) -> BlockValidator<'a> {
        let previous_block = self.load_previous_block();
        let account = self.get_account(&previous_block);
        let account = account.unwrap_or_default();
        let source_block = self.block.source_or_link();
        let source_block_exists = !source_block.is_zero()
            && self
                .ledger
                .any()
                .block_exists_or_pruned(self.txn, &source_block);

        let pending_receive_info = if source_block.is_zero() {
            None
        } else {
            self.ledger
                .any()
                .get_pending(self.txn, &PendingKey::new(account, source_block))
        };

        BlockValidator {
            block: self.block,
            epochs: &self.ledger.constants.epochs,
            work: &self.ledger.constants.work,
            account,
            block_exists: self
                .ledger
                .any()
                .block_exists_or_pruned(self.txn, &self.block.hash()),
            old_account_info: self.ledger.account_info(self.txn, &account),
            pending_receive_info,
            any_pending_exists: self.ledger.any().receivable_exists(self.txn, account),
            source_block_exists,
            previous_block,
            seconds_since_epoch: seconds_since_epoch(),
        }
    }

    fn get_account(&self, previous: &Option<SavedBlock>) -> Option<Account> {
        match self.block.account_field() {
            Some(account) => Some(account),
            None => match previous {
                Some(p) => Some(p.account()),
                None => None,
            },
        }
    }

    fn load_previous_block(&self) -> Option<SavedBlock> {
        if !self.block.previous().is_zero() {
            self.ledger
                .any()
                .get_block(self.txn, &self.block.previous())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsnano_core::{AccountInfo, BlockBuilder, BlockHash, Link, PendingInfo};

    #[test]
    fn block_for_unknown_account() {
        let block = BlockBuilder::state().build();
        let ledger = Ledger::new_null_builder().finish();
        let txn = ledger.read_txn();
        let validator = BlockValidatorFactory::new(&ledger, &txn, &block).create_validator();

        assert_eq!(validator.block.hash(), block.hash());
        assert_eq!(validator.epochs, &ledger.constants.epochs);
        assert_eq!(validator.account, block.account_field().unwrap());
        assert_eq!(validator.block_exists, false);
        assert_eq!(validator.old_account_info, None);
        assert_eq!(validator.pending_receive_info, None);
        assert_eq!(validator.any_pending_exists, false);
        assert_eq!(validator.source_block_exists, false);
        assert_eq!(validator.previous_block, None);
        assert!(validator.seconds_since_epoch >= seconds_since_epoch());
    }

    #[test]
    fn get_account_from_previous_block() {
        let previous = BlockBuilder::legacy_send().build_saved();
        let block = BlockBuilder::legacy_send()
            .previous(previous.hash())
            .build();
        let ledger = Ledger::new_null_builder().block(&previous).finish();
        let txn = ledger.read_txn();
        let validator = BlockValidatorFactory::new(&ledger, &txn, &block).create_validator();

        assert_eq!(validator.account, previous.account());
    }

    #[test]
    fn block_exists() {
        let block = BlockBuilder::state().build_saved();
        let ledger = Ledger::new_null_builder().block(&block).finish();
        let txn = ledger.read_txn();
        let validator = BlockValidatorFactory::new(&ledger, &txn, &block).create_validator();
        assert_eq!(validator.block_exists, true);
    }

    #[test]
    fn pruned_block_exists() {
        let block = BlockBuilder::state().build();
        let ledger = Ledger::new_null_builder().pruned(&block.hash()).finish();
        let txn = ledger.read_txn();
        let validator = BlockValidatorFactory::new(&ledger, &txn, &block).create_validator();
        assert_eq!(validator.block_exists, true);
    }

    #[test]
    fn account_info() {
        let block = BlockBuilder::state().build();
        let account_info = AccountInfo::new_test_instance();
        let ledger = Ledger::new_null_builder()
            .account_info(&block.account_field().unwrap(), &account_info)
            .finish();
        let txn = ledger.read_txn();
        let validator = BlockValidatorFactory::new(&ledger, &txn, &block).create_validator();
        assert_eq!(validator.old_account_info, Some(account_info));
    }

    #[test]
    fn pending_receive_info_for_state_block() {
        let block = BlockBuilder::state().link(Link::from(42)).build();
        let pending_info = PendingInfo::new_test_instance();
        let ledger = Ledger::new_null_builder()
            .pending(
                &PendingKey::new(block.account_field().unwrap(), BlockHash::from(42)),
                &pending_info,
            )
            .finish();
        let txn = ledger.read_txn();
        let validator = BlockValidatorFactory::new(&ledger, &txn, &block).create_validator();
        assert_eq!(validator.pending_receive_info, Some(pending_info));
    }

    #[test]
    fn pending_receive_info_for_legacy_receive() {
        let account = Account::from(1111);
        let previous = BlockBuilder::legacy_open().account(account).build_saved();
        let block = BlockBuilder::legacy_receive()
            .previous(previous.hash())
            .source(BlockHash::from(42))
            .build();
        let pending_info = PendingInfo::new_test_instance();
        let ledger = Ledger::new_null_builder()
            .block(&previous)
            .pending(
                &PendingKey::new(account, BlockHash::from(42)),
                &pending_info,
            )
            .finish();
        let txn = ledger.read_txn();
        let validator = BlockValidatorFactory::new(&ledger, &txn, &block).create_validator();
        assert_eq!(validator.pending_receive_info, Some(pending_info));
    }

    #[test]
    fn any_pending_exists() {
        let block = BlockBuilder::state().build();
        let pending_info = PendingInfo::new_test_instance();
        let ledger = Ledger::new_null_builder()
            .pending(
                &PendingKey::new(block.account_field().unwrap(), BlockHash::from(42)),
                &pending_info,
            )
            .finish();
        let txn = ledger.read_txn();
        let validator = BlockValidatorFactory::new(&ledger, &txn, &block).create_validator();
        assert_eq!(validator.any_pending_exists, true);
    }

    #[test]
    fn source_block_exists() {
        let source = BlockBuilder::state().build_saved();
        let block = BlockBuilder::state().link(source.hash()).build();
        let ledger = Ledger::new_null_builder().block(&source).finish();
        let txn = ledger.read_txn();
        let validator = BlockValidatorFactory::new(&ledger, &txn, &block).create_validator();
        assert_eq!(validator.source_block_exists, true);
    }

    #[test]
    fn pruned_source_block_exists() {
        let block = BlockBuilder::state().link(BlockHash::from(42)).build();
        let ledger = Ledger::new_null_builder()
            .pruned(&BlockHash::from(42))
            .finish();
        let txn = ledger.read_txn();
        let validator = BlockValidatorFactory::new(&ledger, &txn, &block).create_validator();
        assert_eq!(validator.source_block_exists, true);
    }

    #[test]
    fn previous_block() {
        let previous = SavedBlock::new_test_instance();
        let block = BlockBuilder::state()
            .previous(previous.hash())
            .build_saved();
        let ledger = Ledger::new_null_builder().block(&previous).finish();
        let txn = ledger.read_txn();
        let validator = BlockValidatorFactory::new(&ledger, &txn, &block).create_validator();
        assert_eq!(validator.previous_block, Some(previous));
    }
}
