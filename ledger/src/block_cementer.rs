use crate::{LedgerConstants, LedgerObserver, LedgerSetAny, LedgerSetConfirmed};
use rsnano_core::{BlockHash, ConfirmationHeightInfo, SavedBlock};
use rsnano_store_lmdb::{LmdbStore, LmdbWriteTransaction, Transaction};
use std::{collections::VecDeque, sync::atomic::Ordering};

/// Cements Blocks in the ledger
pub(crate) struct BlockCementer<'a> {
    constants: &'a LedgerConstants,
    store: &'a LmdbStore,
    observer: &'a dyn LedgerObserver,
    any: LedgerSetAny<'a>,
    confirmed: LedgerSetConfirmed<'a>,
}

impl<'a> BlockCementer<'a> {
    pub(crate) fn new(
        store: &'a LmdbStore,
        observer: &'a dyn LedgerObserver,
        constants: &'a LedgerConstants,
    ) -> Self {
        Self {
            store,
            observer,
            constants,
            any: LedgerSetAny::new(store),
            confirmed: LedgerSetConfirmed::new(store),
        }
    }

    pub(crate) fn confirm(
        &self,
        txn: &mut LmdbWriteTransaction,
        target_hash: BlockHash,
        max_blocks: usize,
    ) -> Vec<SavedBlock> {
        let mut result = Vec::new();

        let mut stack = VecDeque::new();
        stack.push_back(target_hash);
        while let Some(&hash) = stack.back() {
            let block = self.any.get_block(txn, &hash).unwrap();

            let dependents =
                block.dependent_blocks(&self.constants.epochs, &self.constants.genesis_account);
            for dependent in dependents.iter() {
                if !dependent.is_zero() && !self.confirmed.block_exists_or_pruned(txn, dependent) {
                    self.observer.dependent_unconfirmed();

                    stack.push_back(*dependent);

                    // Limit the stack size to avoid excessive memory usage
                    // This will forget the bottom of the dependency tree
                    if stack.len() > max_blocks {
                        stack.pop_front();
                    }
                }
            }

            if stack.back() == Some(&hash) {
                stack.pop_back();
                if !self.confirmed.block_exists_or_pruned(txn, &hash) {
                    // We must only confirm blocks that have their dependencies confirmed

                    let conf_height = ConfirmationHeightInfo::new(block.height(), block.hash());

                    // Update store
                    self.store
                        .confirmation_height
                        .put(txn, &block.account(), &conf_height);
                    self.store
                        .cache
                        .cemented_count
                        .fetch_add(1, Ordering::SeqCst);

                    self.observer.blocks_cemented(1);

                    result.push(block);
                }
            } else {
                // Unconfirmed dependencies were added
            }

            // Refresh the transaction to avoid long-running transactions
            // Ensure that the block wasn't rolled back during the refresh
            let refreshed = txn.refresh_if_needed();
            if refreshed {
                if !self.any.block_exists(txn, &target_hash) {
                    break; // Block was rolled back during cementing
                }
            }

            // Early return might leave parts of the dependency tree unconfirmed
            if result.len() >= max_blocks {
                break;
            }
        }
        result
    }
}
