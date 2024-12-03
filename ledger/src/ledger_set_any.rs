use rsnano_core::{
    utils::{BufferReader, Deserialize},
    Account, AccountInfo, Amount, BlockHash, PendingInfo, PendingKey, QualifiedRoot, SavedBlock,
};
use rsnano_store_lmdb::{LmdbIterator, LmdbPendingStore, LmdbStore, Transaction};
use std::ops::{Deref, RangeBounds};

pub struct LedgerSetAny<'a> {
    store: &'a LmdbStore,
}

impl<'a> LedgerSetAny<'a> {
    pub fn new(store: &'a LmdbStore) -> Self {
        Self { store }
    }

    pub fn get_block(&self, tx: &dyn Transaction, hash: &BlockHash) -> Option<SavedBlock> {
        self.store.block.get(tx, hash)
    }

    pub fn get_account(&self, tx: &dyn Transaction, account: &Account) -> Option<AccountInfo> {
        self.store.account.get(tx, account)
    }

    pub fn account_head(&self, tx: &dyn Transaction, account: &Account) -> Option<BlockHash> {
        self.get_account(tx, account).map(|i| i.head)
    }

    pub fn account_balance(&self, tx: &dyn Transaction, account: &Account) -> Option<Amount> {
        let head = self.account_head(tx, account)?;
        self.get_block(tx, &head).map(|b| b.balance())
    }

    pub fn account_height(&self, tx: &dyn Transaction, account: &Account) -> u64 {
        let Some(head) = self.account_head(tx, account) else {
            return 0;
        };
        self.get_block(tx, &head)
            .map(|b| b.height())
            .expect("Head block not in ledger!")
    }

    pub fn block_account(&self, tx: &dyn Transaction, hash: &BlockHash) -> Option<Account> {
        self.get_block(tx, hash).map(|b| b.account())
    }

    pub fn block_amount(&self, tx: &dyn Transaction, hash: &BlockHash) -> Option<Amount> {
        let block = self.get_block(tx, hash)?;
        self.block_amount_for(tx, &block)
    }

    pub fn block_amount_for(&self, tx: &dyn Transaction, block: &SavedBlock) -> Option<Amount> {
        let block_balance = block.balance();
        if block.previous().is_zero() {
            Some(block_balance)
        } else {
            let previous_balance = self.block_balance(tx, &block.previous())?;
            if block_balance > previous_balance {
                Some(block_balance - previous_balance)
            } else {
                Some(previous_balance - block_balance)
            }
        }
    }

    pub fn block_balance(&self, tx: &dyn Transaction, hash: &BlockHash) -> Option<Amount> {
        if hash.is_zero() {
            return None;
        }

        self.get_block(tx, hash).map(|b| b.balance())
    }

    pub fn block_exists(&self, tx: &dyn Transaction, hash: &BlockHash) -> bool {
        self.store.block.exists(tx, hash)
    }

    pub fn block_exists_or_pruned(&self, tx: &dyn Transaction, hash: &BlockHash) -> bool {
        if self.store.pruned.exists(tx, hash) {
            true
        } else {
            self.store.block.exists(tx, hash)
        }
    }

    pub fn block_height(&self, tx: &dyn Transaction, hash: &BlockHash) -> u64 {
        self.get_block(tx, hash)
            .map(|b| b.height())
            .unwrap_or_default()
    }

    pub fn block_successor(&self, tx: &dyn Transaction, hash: &BlockHash) -> Option<BlockHash> {
        self.block_successor_by_qualified_root(tx, &QualifiedRoot::new(hash.into(), *hash))
    }

    pub fn block_successor_by_qualified_root(
        &self,
        tx: &dyn Transaction,
        root: &QualifiedRoot,
    ) -> Option<BlockHash> {
        if !root.previous.is_zero() {
            self.store.block.successor(tx, &root.previous)
        } else {
            self.get_account(tx, &root.root.into())
                .map(|i| i.open_block)
        }
    }

    pub fn get_pending(&self, tx: &dyn Transaction, key: &PendingKey) -> Option<PendingInfo> {
        self.store.pending.get(tx, key)
    }

    /// Returns the next receivable entry for the account 'account' with hash greater than 'hash'
    pub fn account_receivable_upper_bound<'txn>(
        &self,
        txn: &'txn dyn Transaction,
        account: Account,
        hash: BlockHash,
    ) -> AnyReceivableIterator<'txn>
    where
        'a: 'txn,
    {
        AnyReceivableIterator::<'txn>::new(
            txn,
            self.store.pending.deref(),
            account,
            Some(account),
            hash.inc(),
        )
    }

    /// Returns the next receivable entry for an account greater than 'account'
    pub fn receivable_upper_bound<'txn>(
        &self,
        txn: &'txn dyn Transaction,
        account: Account,
    ) -> AnyReceivableIterator<'txn>
    where
        'a: 'txn,
    {
        match account.inc() {
            None => AnyReceivableIterator::<'txn>::new(
                txn,
                self.store.pending.deref(),
                Default::default(),
                None,
                None,
            ),
            Some(account) => AnyReceivableIterator::<'txn>::new(
                txn,
                self.store.pending.deref(),
                account,
                None,
                Some(BlockHash::zero()),
            ),
        }
    }

    /// Returns the next receivable entry for an account greater than or equal to 'account'
    pub fn receivable_lower_bound<'txn>(
        &'a self,
        txn: &'a dyn Transaction,
        account: Account,
    ) -> AnyReceivableIterator<'txn>
    where
        'a: 'txn,
    {
        AnyReceivableIterator::<'txn>::new(
            txn,
            self.store.pending.deref(),
            account,
            None,
            Some(BlockHash::zero()),
        )
    }

    pub fn receivable_exists(&self, txn: &dyn Transaction, account: Account) -> bool {
        self.account_receivable_upper_bound(txn, account, BlockHash::zero())
            .next()
            .is_some()
    }

    pub fn accounts<'txn>(
        &self,
        tx: &'txn dyn Transaction,
    ) -> impl Iterator<Item = (Account, AccountInfo)> + 'txn {
        self.store.account.iter(tx)
    }

    pub fn accounts_range<'txn>(
        &self,
        tx: &'txn dyn Transaction,
        range: impl RangeBounds<Account> + 'static,
    ) -> impl Iterator<Item = (Account, AccountInfo)> + 'txn {
        self.store.account.iter_range(tx, range)
    }
}

pub struct AnyReceivableIterator<'a> {
    pub txn: &'a dyn Transaction,
    pub pending: &'a LmdbPendingStore,
    pub requested_account: Account,
    pub actual_account: Option<Account>,
    pub next_hash: Option<BlockHash>,
    inner: LmdbIterator<'a, PendingKey, PendingInfo>,
    is_first: bool,
}

impl<'a> AnyReceivableIterator<'a> {
    pub fn new(
        txn: &'a dyn Transaction,
        pending: &'a LmdbPendingStore,
        requested_account: Account,
        actual_account: Option<Account>,
        next_hash: Option<BlockHash>,
    ) -> Self {
        let cursor = txn
            .open_ro_cursor(pending.database())
            .expect("could not read from account store");

        Self {
            txn,
            requested_account,
            actual_account,
            next_hash,
            pending,
            inner: LmdbIterator::new(cursor, read_pending_entry),
            is_first: true,
        }
    }
}

fn read_pending_entry(key: &[u8], value: &[u8]) -> (PendingKey, PendingInfo) {
    let mut stream = BufferReader::new(key);
    let key = PendingKey::deserialize(&mut stream).unwrap();
    let mut stream = BufferReader::new(value);
    let info = PendingInfo::deserialize(&mut stream).unwrap();
    (key, info)
}

impl<'a> Iterator for AnyReceivableIterator<'a> {
    type Item = (PendingKey, PendingInfo);

    fn next(&mut self) -> Option<Self::Item> {
        let hash = self.next_hash?;
        if self.is_first {
            self.is_first = false;
            let (key, info) = self
                .inner
                .start_at(&PendingKey::new(self.requested_account, hash))?;
            if let Some(actual) = self.actual_account {
                if actual != key.receiving_account {
                    return None;
                }
            }
            self.actual_account = Some(key.receiving_account);
            return Some((key, info));
        }

        let (key, info) = self.inner.next()?;
        match self.actual_account {
            Some(account) => {
                if key.receiving_account == account {
                    self.next_hash = key.send_block_hash.inc();
                    Some((key, info))
                } else {
                    None
                }
            }
            None => {
                self.actual_account = Some(key.receiving_account);
                Some((key, info))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Ledger;

    #[test]
    fn iter_all_lower_bound() {
        let key1 = PendingKey::new(Account::from(1), BlockHash::from(100));
        let key2 = PendingKey::new(Account::from(1), BlockHash::from(101));
        let key3 = PendingKey::new(Account::from(3), BlockHash::from(4));

        test_lower_bound(
            &[key1.clone(), key2.clone(), key3.clone()],
            Account::from(0),
            &[key1.clone(), key2.clone()],
        );
        test_lower_bound(
            &[key1.clone(), key2.clone(), key3.clone()],
            Account::from(1),
            &[key1.clone(), key2.clone()],
        );
        test_lower_bound(
            &[key1.clone(), key2.clone(), key3.clone()],
            Account::from(3),
            &[key3.clone()],
        );
        test_lower_bound(
            &[key1.clone(), key2.clone(), key3.clone()],
            Account::from(4),
            &[],
        );
    }

    #[test]
    fn iter_all_upper_bound() {
        let key1 = PendingKey::new(Account::from(1), BlockHash::from(100));
        let key2 = PendingKey::new(Account::from(1), BlockHash::from(101));
        let key3 = PendingKey::new(Account::from(3), BlockHash::from(4));
        test_upper_bound(
            &[key1.clone(), key2.clone(), key3.clone()],
            Account::from(0),
            &[key1.clone(), key2.clone()],
        );
        test_upper_bound(
            &[key1.clone(), key2.clone(), key3.clone()],
            Account::from(1),
            &[key3.clone()],
        );
        test_upper_bound(
            &[key1.clone(), key2.clone(), key3.clone()],
            Account::from(4),
            &[],
        );
    }

    fn test_upper_bound(
        existing_keys: &[PendingKey],
        queried_account: Account,
        expected_result: &[PendingKey],
    ) {
        let ledger = ledger_with_pending_entries(existing_keys);
        let tx = ledger.read_txn();
        let result: Vec<_> = ledger
            .any()
            .receivable_upper_bound(&tx, queried_account)
            .map(|(k, _)| k)
            .collect();

        assert_eq!(result, expected_result);
    }

    fn test_lower_bound(
        existing_keys: &[PendingKey],
        queried_account: Account,
        expected_result: &[PendingKey],
    ) {
        let ledger = ledger_with_pending_entries(existing_keys);
        let tx = ledger.read_txn();
        let result: Vec<_> = ledger
            .any()
            .receivable_lower_bound(&tx, queried_account)
            .map(|(k, _)| k)
            .collect();

        assert_eq!(result, expected_result);
    }

    fn ledger_with_pending_entries(existing_keys: &[PendingKey]) -> Ledger {
        let info = PendingInfo::new_test_instance();
        let mut builder = Ledger::new_null_builder();
        for key in existing_keys {
            builder = builder.pending(key, &info);
        }
        builder.finish()
    }
}
