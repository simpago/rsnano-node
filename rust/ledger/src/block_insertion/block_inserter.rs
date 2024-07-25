use crate::Ledger;
use rsnano_core::{
    Account, AccountInfo, Amount, BlockEnum, BlockSideband, PendingInfo, PendingKey,
};
use rsnano_store_lmdb::LmdbWriteTransaction;
use std::sync::atomic::Ordering;

#[derive(Debug, PartialEq)]
pub(crate) struct BlockInsertInstructions {
    pub account: Account,
    pub old_account_info: AccountInfo,
    pub set_account_info: AccountInfo,
    pub delete_pending: Option<PendingKey>,
    pub insert_pending: Option<(PendingKey, PendingInfo)>,
    pub set_sideband: BlockSideband,
    pub is_epoch_block: bool,
}

/// Inserts a new block into the ledger
pub(crate) struct BlockInserter<'a> {
    ledger: &'a Ledger,
    txn: &'a mut LmdbWriteTransaction,
    block: &'a mut BlockEnum,
    instructions: &'a BlockInsertInstructions,
}

impl<'a> BlockInserter<'a> {
    pub(crate) fn new(
        ledger: &'a Ledger,
        txn: &'a mut LmdbWriteTransaction,
        block: &'a mut BlockEnum,
        instructions: &'a BlockInsertInstructions,
    ) -> Self {
        Self {
            ledger,
            txn,
            block,
            instructions,
        }
    }

    pub(crate) fn insert(&mut self) {
        self.set_block_sideband();
        self.ledger.store.block.put(self.txn, self.block);
        self.update_account();
        self.delete_old_pending_info();
        self.insert_new_pending_info();
        self.update_representative_cache();
        self.ledger
            .observer
            .block_added(self.block, self.instructions.is_epoch_block);
        self.ledger
            .store
            .cache
            .block_count
            .fetch_add(1, Ordering::SeqCst);
    }

    fn set_block_sideband(&mut self) {
        self.block
            .set_sideband(self.instructions.set_sideband.clone());
    }

    fn update_account(&mut self) {
        self.ledger.update_account(
            self.txn,
            &self.instructions.account,
            &self.instructions.old_account_info,
            &self.instructions.set_account_info,
        );
    }

    fn delete_old_pending_info(&mut self) {
        if let Some(key) = &self.instructions.delete_pending {
            self.ledger.store.pending.del(self.txn, key);
        }
    }

    fn insert_new_pending_info(&mut self) {
        if let Some((key, info)) = &self.instructions.insert_pending {
            self.ledger.store.pending.put(self.txn, key, info);
        }
    }

    fn update_representative_cache(&mut self) {
        if !self.instructions.old_account_info.head.is_zero() {
            // Move existing representation & add in amount delta
            self.ledger.rep_weights_updater.representation_add_dual(
                self.txn,
                self.instructions.old_account_info.representative,
                Amount::zero().wrapping_sub(self.instructions.old_account_info.balance),
                self.instructions.set_account_info.representative,
                self.instructions.set_account_info.balance,
            );
        } else {
            // Add in amount delta only
            self.ledger.rep_weights_updater.representation_add(
                self.txn,
                self.instructions.set_account_info.representative,
                self.instructions.set_account_info.balance,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsnano_core::{BlockBuilder, BlockHash};

    #[test]
    fn insert_open_state_block() {
        let (mut block, instructions) = open_state_block_instructions();
        let ledger = Ledger::new_null();

        let result = insert(&ledger, &mut block, &instructions);

        assert_eq!(block.sideband().unwrap(), &instructions.set_sideband);
        assert_eq!(result.saved_blocks, vec![block]);
        assert_eq!(
            result.saved_accounts,
            vec![(instructions.account, instructions.set_account_info.clone())]
        );
        assert_eq!(
            ledger
                .rep_weights
                .weight(&instructions.set_account_info.representative),
            instructions.set_account_info.balance
        );
        assert_eq!(ledger.store.cache.block_count.load(Ordering::Relaxed), 1);
        assert_eq!(result.deleted_pending, Vec::new());
    }

    #[test]
    fn delete_old_pending() {
        let (mut block, mut instructions) = legacy_open_block_instructions();
        let pending_key = PendingKey::new_test_instance();
        instructions.delete_pending = Some(pending_key.clone());
        let ledger = Ledger::new_null();

        let result = insert(&ledger, &mut block, &instructions);

        assert_eq!(result.deleted_pending, vec![pending_key]);
    }

    #[test]
    fn insert_pending() {
        let (mut block, mut instructions) = legacy_open_block_instructions();
        let pending_key = PendingKey::new_test_instance();
        let pending_info = PendingInfo::new_test_instance();
        instructions.insert_pending = Some((pending_key.clone(), pending_info.clone()));
        let ledger = Ledger::new_null();

        let result = insert(&ledger, &mut block, &instructions);

        assert_eq!(result.saved_pending, vec![(pending_key, pending_info)]);
    }

    #[test]
    fn update_representative() {
        let old_representative = Account::from(1111);
        let new_representative = Account::from(2222);
        let mut open = BlockBuilder::legacy_open()
            .representative(old_representative)
            .build();
        let sideband = BlockSideband {
            successor: BlockHash::zero(),
            ..BlockSideband::new_test_instance()
        };
        open.set_sideband(sideband.clone());

        let state = BlockBuilder::state()
            .previous(open.hash())
            .representative(new_representative)
            .balance(sideband.balance)
            .build();
        let (mut state, instructions) = state_block_instructions_for(&open, state);

        let ledger = Ledger::new_null_builder().block(&open).finish();
        insert(&ledger, &mut state, &instructions);
        assert_eq!(
            ledger.rep_weights.weight(&new_representative),
            instructions.set_account_info.balance
        );
    }

    fn insert(
        ledger: &Ledger,
        block: &mut BlockEnum,
        instructions: &BlockInsertInstructions,
    ) -> InsertResult {
        let mut txn = ledger.rw_txn();
        let saved_blocks = ledger.store.block.track_puts();
        let saved_accounts = ledger.store.account.track_puts();
        let saved_pending = ledger.store.pending.track_puts();
        let deleted_pending = ledger.store.pending.track_deletions();

        let mut block_inserter = BlockInserter::new(&ledger, &mut txn, block, &instructions);
        block_inserter.insert();

        InsertResult {
            saved_blocks: saved_blocks.output(),
            saved_accounts: saved_accounts.output(),
            saved_pending: saved_pending.output(),
            deleted_pending: deleted_pending.output(),
        }
    }

    struct InsertResult {
        saved_blocks: Vec<BlockEnum>,
        saved_accounts: Vec<(Account, AccountInfo)>,
        saved_pending: Vec<(PendingKey, PendingInfo)>,
        deleted_pending: Vec<PendingKey>,
    }

    fn legacy_open_block_instructions() -> (BlockEnum, BlockInsertInstructions) {
        let block = BlockBuilder::legacy_open().build();
        let sideband = BlockSideband {
            successor: BlockHash::zero(),
            ..BlockSideband::new_test_instance()
        };
        let account_info = AccountInfo {
            head: block.hash(),
            open_block: block.hash(),
            ..AccountInfo::new_test_instance()
        };
        let instructions = BlockInsertInstructions {
            account: block.account_field().unwrap(),
            old_account_info: AccountInfo::default(),
            set_account_info: account_info,
            delete_pending: None,
            insert_pending: None,
            set_sideband: sideband,
            is_epoch_block: false,
        };

        (block, instructions)
    }

    fn open_state_block_instructions() -> (BlockEnum, BlockInsertInstructions) {
        let block = BlockBuilder::state().previous(BlockHash::zero()).build();
        let sideband = BlockSideband {
            successor: BlockHash::zero(),
            ..BlockSideband::new_test_instance()
        };
        let account_info = AccountInfo {
            head: block.hash(),
            open_block: block.hash(),
            ..AccountInfo::new_test_instance()
        };
        let instructions = BlockInsertInstructions {
            account: Account::from(1),
            old_account_info: AccountInfo::default(),
            set_account_info: account_info,
            delete_pending: None,
            insert_pending: None,
            set_sideband: sideband,
            is_epoch_block: false,
        };

        (block, instructions)
    }

    fn state_block_instructions_for(
        previous: &BlockEnum,
        block: BlockEnum,
    ) -> (BlockEnum, BlockInsertInstructions) {
        let sideband = BlockSideband {
            successor: BlockHash::zero(),
            balance: block.balance(),
            account: block.account_field().unwrap(),
            ..BlockSideband::new_test_instance()
        };
        let old_account_info = AccountInfo {
            head: previous.hash(),
            balance: previous.balance(),
            ..AccountInfo::new_test_instance()
        };
        let new_account_info = AccountInfo {
            head: block.hash(),
            open_block: block.hash(),
            balance: block.balance_field().unwrap(),
            representative: block.representative_field().unwrap(),
            ..AccountInfo::new_test_instance()
        };
        let instructions = BlockInsertInstructions {
            account: Account::from(1),
            old_account_info,
            set_account_info: new_account_info,
            delete_pending: None,
            insert_pending: None,
            set_sideband: sideband,
            is_epoch_block: false,
        };

        (block, instructions)
    }
}
