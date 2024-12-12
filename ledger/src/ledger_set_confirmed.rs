use rsnano_core::{Account, Amount, BlockHash, PendingInfo, PendingKey, SavedBlock};
use rsnano_store_lmdb::{LmdbStore, Transaction};

pub struct LedgerSetConfirmed<'a> {
    store: &'a LmdbStore,
}

impl<'a> LedgerSetConfirmed<'a> {
    pub fn new(store: &'a LmdbStore) -> Self {
        Self { store }
    }

    pub fn get_block(&self, tx: &dyn Transaction, hash: &BlockHash) -> Option<SavedBlock> {
        let block = self.store.block.get(tx, hash)?;
        let info = self.store.confirmation_height.get(tx, &block.account())?;
        if block.height() <= info.height {
            Some(block)
        } else {
            None
        }
    }

    pub fn account_head(&self, tx: &dyn Transaction, account: &Account) -> Option<BlockHash> {
        let info = self.store.confirmation_height.get(tx, account)?;
        Some(info.frontier)
    }

    pub fn account_height(&self, tx: &dyn Transaction, account: &Account) -> u64 {
        let Some(head) = self.account_head(tx, account) else {
            return 0;
        };
        self.get_block(tx, &head)
            .map(|b| b.height())
            .expect("Head block not in ledger!")
    }

    pub fn block_balance(&self, tx: &dyn Transaction, hash: &BlockHash) -> Option<Amount> {
        if hash.is_zero() {
            return None;
        }

        self.get_block(tx, hash).map(|b| b.balance())
    }

    pub fn block_exists(&self, tx: &dyn Transaction, hash: &BlockHash) -> bool {
        self.get_block(tx, hash).is_some()
    }

    pub fn block_exists_or_pruned(&self, tx: &dyn Transaction, hash: &BlockHash) -> bool {
        if self.store.pruned.exists(tx, hash) {
            true
        } else {
            self.block_exists(tx, hash)
        }
    }

    pub fn account_balance(&self, tx: &dyn Transaction, account: &Account) -> Option<Amount> {
        let head = self.account_head(tx, account)?;
        self.get_block(tx, &head).map(|b| b.balance())
    }

    /// Returns the next receivable entry for an account greater than or equal to 'account'
    pub fn receivable_lower_bound<'txn>(
        &'a self,
        txn: &'a dyn Transaction,
        account: Account,
    ) -> ConfirmedReceivableIterator<'txn>
    where
        'a: 'txn,
    {
        ConfirmedReceivableIterator::<'txn> {
            txn,
            set: self,
            requested_account: account,
            actual_account: None,
            next_hash: Some(BlockHash::zero()),
        }
    }

    fn first_receivable_lower_bound(
        &self,
        txn: &dyn Transaction,
        account: Account,
        send_hash: BlockHash,
    ) -> Option<(PendingKey, PendingInfo)> {
        let mut it = self
            .store
            .pending
            .iter_range(txn, PendingKey::new(account, send_hash)..);

        let (mut key, mut info) = it.next()?;

        while !self.block_exists(txn, &key.send_block_hash) {
            (key, info) = it.next()?;
        }

        Some((key, info))
    }
}

pub struct ConfirmedReceivableIterator<'a> {
    pub txn: &'a dyn Transaction,
    pub set: &'a LedgerSetConfirmed<'a>,
    pub requested_account: Account,
    pub actual_account: Option<Account>,
    pub next_hash: Option<BlockHash>,
}

impl<'a> Iterator for ConfirmedReceivableIterator<'a> {
    type Item = (PendingKey, PendingInfo);

    fn next(&mut self) -> Option<Self::Item> {
        let hash = self.next_hash?;
        let account = self.actual_account.unwrap_or(self.requested_account);
        let (key, info) = self
            .set
            .first_receivable_lower_bound(self.txn, account, hash)?;
        match self.actual_account {
            Some(account) => {
                if key.receiving_account == account {
                    self.next_hash = key.send_block_hash.inc();
                    Some((key.clone(), info.clone()))
                } else {
                    None
                }
            }
            None => {
                self.actual_account = Some(key.receiving_account);
                self.next_hash = key.send_block_hash.inc();
                Some((key.clone(), info.clone()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Ledger;
    use rsnano_core::{
        Account, BlockHash, ConfirmationHeightInfo, PendingInfo, PendingKey, SavedBlock,
    };

    #[test]
    fn iter_receivables() {
        let account = Account::from(1);

        let block1 = SavedBlock::new_test_instance_with_key(42);
        let block2 = SavedBlock::new_test_instance_with_key(43);
        let block3 = SavedBlock::new_test_instance_with_key(44);

        let ledger = Ledger::new_null_builder()
            .blocks([&block1, &block2, &block3])
            .confirmation_height(
                &block1.account(),
                &ConfirmationHeightInfo::new(9999, BlockHash::zero()),
            )
            .confirmation_height(
                &block2.account(),
                &ConfirmationHeightInfo::new(0, BlockHash::zero()),
            )
            .confirmation_height(
                &block3.account(),
                &ConfirmationHeightInfo::new(9999, BlockHash::zero()),
            )
            .pending(
                &PendingKey::new(account, block1.hash()),
                &PendingInfo::new_test_instance(),
            )
            .pending(
                &PendingKey::new(account, block2.hash()),
                &PendingInfo::new_test_instance(),
            )
            .pending(
                &PendingKey::new(account, block3.hash()),
                &PendingInfo::new_test_instance(),
            )
            .finish();

        let tx = ledger.read_txn();
        let receivable: Vec<_> = ledger
            .confirmed()
            .receivable_lower_bound(&tx, Account::zero())
            .map(|i| i.0)
            .collect();

        let mut expected = vec![
            PendingKey::new(account, block1.hash()),
            PendingKey::new(account, block3.hash()),
        ];
        expected.sort_by_key(|i| i.send_block_hash);

        assert_eq!(receivable, expected);
    }
}
