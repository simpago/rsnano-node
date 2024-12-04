use super::rollback_planner::RollbackInstructions;
use crate::Ledger;
use rsnano_core::{Amount, PublicKey};
use rsnano_store_lmdb::LmdbWriteTransaction;
use std::sync::atomic::Ordering;

/// Updates the ledger according to the RollbackInstructions
pub(crate) struct RollbackInstructionsExecutor<'a> {
    ledger: &'a Ledger,
    txn: &'a mut LmdbWriteTransaction,
    instructions: &'a RollbackInstructions,
}

impl<'a> RollbackInstructionsExecutor<'a> {
    pub(crate) fn new(
        ledger: &'a Ledger,
        txn: &'a mut LmdbWriteTransaction,
        instructions: &'a RollbackInstructions,
    ) -> Self {
        Self {
            ledger,
            txn,
            instructions,
        }
    }

    pub(crate) fn execute(&mut self) {
        self.update_pending_table();
        self.update_account_table();
        self.update_block_table();
        self.roll_back_representative_cache();
        self.ledger
            .store
            .cache
            .block_count
            .fetch_sub(1, Ordering::SeqCst);

        self.ledger
            .observer
            .block_rolled_back(self.instructions.block_sub_type);
    }

    fn update_block_table(&mut self) {
        self.ledger
            .store
            .block
            .del(self.txn, &self.instructions.block_hash);

        if let Some(hash) = self.instructions.clear_successor {
            self.ledger.store.block.successor_clear(self.txn, &hash);
        }
    }

    fn update_account_table(&mut self) {
        self.ledger.update_account(
            self.txn,
            &self.instructions.account,
            &self.instructions.old_account_info,
            &self.instructions.set_account_info,
        );
    }

    fn update_pending_table(&mut self) {
        if let Some(pending_key) = &self.instructions.remove_pending {
            self.ledger.store.pending.del(self.txn, pending_key);
        }
        if let Some((key, info)) = &self.instructions.add_pending {
            self.ledger.store.pending.put(self.txn, key, info);
        }
    }

    fn roll_back_representative_cache(&mut self) {
        if let Some(previous_rep) = &self.instructions.new_representative {
            self.roll_back_change_in_representative_cache(previous_rep);
        } else {
            self.roll_back_receive_in_representative_cache()
        }
    }

    fn roll_back_change_in_representative_cache(&mut self, previous_representative: &PublicKey) {
        self.ledger.rep_weights_updater.representation_add_dual(
            self.txn,
            self.instructions.old_account_info.representative,
            Amount::zero().wrapping_sub(self.instructions.old_account_info.balance),
            *previous_representative,
            self.instructions.new_balance,
        );
    }

    fn roll_back_receive_in_representative_cache(&mut self) {
        self.ledger.rep_weights_updater.representation_add(
            self.txn,
            self.instructions.old_account_info.representative,
            Amount::zero().wrapping_sub(self.instructions.old_account_info.balance),
        );
    }
}
