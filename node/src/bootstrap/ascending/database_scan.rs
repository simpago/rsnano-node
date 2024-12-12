use rsnano_core::{utils::ContainerInfo, Account, BlockHash, PendingKey};
use rsnano_ledger::Ledger;
use rsnano_store_lmdb::LmdbReadTransaction;
use std::{collections::VecDeque, sync::Arc};

const BATCH_SIZE: usize = 512;

pub(crate) struct DatabaseScan {
    queue: VecDeque<Account>,
    accounts_iterator: AccountDatabaseIterator,
    pending_iterator: PendingDatabaseIterator,
    ledger: Arc<Ledger>,
}

impl DatabaseScan {
    pub fn new(ledger: Arc<Ledger>) -> Self {
        Self {
            accounts_iterator: AccountDatabaseIterator::new(ledger.clone()),
            pending_iterator: PendingDatabaseIterator::new(ledger.clone()),
            ledger,
            queue: Default::default(),
        }
    }

    pub fn next(&mut self, filter: impl Fn(&Account) -> bool) -> Account {
        if self.queue.is_empty() {
            self.fill();
        }

        while let Some(result) = self.queue.pop_front() {
            if filter(&result) {
                return result;
            }
        }

        Account::zero()
    }

    fn fill(&mut self) {
        let tx = self.ledger.read_txn();
        let set1 = self.accounts_iterator.next_batch(&tx, BATCH_SIZE);
        let set2 = self.pending_iterator.next_batch(&tx, BATCH_SIZE);
        self.queue.extend(set1);
        self.queue.extend(set2);
    }

    pub fn warmed_up(&self) -> bool {
        self.accounts_iterator.warmed_up() && self.pending_iterator.warmed_up()
    }

    pub fn container_info(&self) -> ContainerInfo {
        [
            ("accounts_iterator", self.accounts_iterator.completed, 0),
            ("pending_iterator", self.pending_iterator.completed, 0),
        ]
        .into()
    }
}

struct AccountDatabaseIterator {
    ledger: Arc<Ledger>,
    next: Account,
    completed: usize,
}

impl AccountDatabaseIterator {
    fn new(ledger: Arc<Ledger>) -> Self {
        Self {
            ledger,
            next: Account::zero(),
            completed: 0,
        }
    }

    fn next_batch(&mut self, tx: &LmdbReadTransaction, batch_size: usize) -> Vec<Account> {
        let mut result = Vec::new();
        let accounts = self.ledger.store.account.iter_range(tx, self.next..);
        let mut count = 0;
        let mut end_reached = true;
        for (account, _) in accounts {
            if count >= batch_size {
                end_reached = false;
                break;
            }
            result.push(account);
            count += 1;
            self.next = account.inc().unwrap_or_default();
        }

        if end_reached {
            // Reset for the next ledger iteration
            self.next = Account::zero();
            self.completed += 1;
        }

        result
    }

    fn warmed_up(&self) -> bool {
        self.completed > 0
    }
}

struct PendingDatabaseIterator {
    ledger: Arc<Ledger>,
    next: PendingKey,
    completed: usize,
}

impl PendingDatabaseIterator {
    fn new(ledger: Arc<Ledger>) -> Self {
        Self {
            ledger,
            next: PendingKey::default(),
            completed: 0,
        }
    }

    fn next_batch(&mut self, tx: &LmdbReadTransaction, batch_size: usize) -> Vec<Account> {
        let mut result = Vec::new();
        let mut it = self
            .ledger
            .store
            .pending
            .iter_range(tx, self.next.clone()..);
        // TODO: This pending iteration heuristic should be encapsulated in a pending_iterator class and reused across other components
        // The heuristic is to advance the iterator sequentially until we reach a new account or perform a fresh lookup if the account has too many pending blocks
        // This is to avoid the overhead of performing a fresh lookup for every pending account as majority of accounts have only a few pending blocks

        let mut count = 0;
        let mut end_reached = true;
        let mut current = it.next();
        while let Some((key, _)) = current {
            if count >= batch_size {
                end_reached = false;
                break;
            }
            result.push(key.receiving_account);
            self.next = PendingKey::new(
                key.receiving_account.inc().unwrap_or_default(),
                BlockHash::zero(),
            );
            count += 1;

            // advance iterator:
            let starting_account = key.receiving_account;

            // For RocksDB, sequential access is ~10x faster than performing a fresh lookup (tested on my machine)
            const SEQUENTIAL_ATTEMPTS: usize = 10;

            let mut found = false;
            for _ in 0..SEQUENTIAL_ATTEMPTS {
                current = it.next();
                match &current {
                    Some((k, _)) => {
                        if k.receiving_account != starting_account {
                            found = true;
                            break;
                        }
                    }
                    None => {
                        break;
                    }
                }
            }

            // If we didn't advance to the next account, perform a fresh lookup
            if !found {
                it = self.ledger.store.pending.iter_range(
                    tx,
                    PendingKey::new(
                        starting_account.inc().unwrap_or_default(),
                        BlockHash::zero(),
                    )..,
                );
                current = it.next();
            }
        }

        if end_reached {
            // Reset for the next ledger iteration
            self.next = Default::default();
            self.completed += 1;
        }
        result
    }

    fn warmed_up(&self) -> bool {
        self.completed > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsnano_core::{PrivateKey, UnsavedBlockLatticeBuilder};
    use rsnano_ledger::LedgerContext;

    #[test]
    fn pending_database_scanner() {
        // Prepare pending sends from genesis
        // 1 account with 1 pending
        // 1 account with 21 pendings
        // 2 accounts with 1 pending each
        let mut lattice = UnsavedBlockLatticeBuilder::new();
        let mut blocks = Vec::new();
        let key1 = PrivateKey::from(1);
        let key2 = PrivateKey::from(2);
        let key3 = PrivateKey::from(3);
        let key4 = PrivateKey::from(4);
        {
            // 1 account with 1 pending
            blocks.push(lattice.genesis().send(&key1, 1));

            // 1 account with 21 pendings
            for _ in 0..21 {
                blocks.push(lattice.genesis().send(&key2, 1));
            }
            // 2 accounts with 1 pending each
            blocks.push(lattice.genesis().send(&key3, 1));
            blocks.push(lattice.genesis().send(&key4, 1));

            let ledger_ctx = LedgerContext::empty_dev();
            for mut block in blocks {
                let mut txn = ledger_ctx.ledger.rw_txn();
                ledger_ctx.ledger.process(&mut txn, &mut block).unwrap();
            }
            // Single batch
            {
                let mut scanner = PendingDatabaseIterator::new(ledger_ctx.ledger.clone());
                let tx = ledger_ctx.ledger.read_txn();
                let accounts = scanner.next_batch(&tx, 256);

                // Check that account set contains all keys
                assert_eq!(accounts.len(), 4);
                assert!(accounts.contains(&key1.account()));
                assert!(accounts.contains(&key2.account()));
                assert!(accounts.contains(&key3.account()));
                assert!(accounts.contains(&key4.account()));

                assert_eq!(scanner.completed, 1);
            }

            // Multi batch
            {
                let mut scanner = PendingDatabaseIterator::new(ledger_ctx.ledger.clone());
                let tx = ledger_ctx.ledger.read_txn();

                // Request accounts in multiple batches
                let accounts1 = scanner.next_batch(&tx, 2);
                let accounts2 = scanner.next_batch(&tx, 1);
                let accounts3 = scanner.next_batch(&tx, 1);

                assert_eq!(accounts1.len(), 2);
                assert_eq!(accounts2.len(), 1);
                assert_eq!(accounts3.len(), 1);

                // Check that account set contains all keys
                let mut accounts = accounts1;
                accounts.extend(accounts2);
                accounts.extend(accounts3);
                assert!(accounts.contains(&key1.account()));
                assert!(accounts.contains(&key2.account()));
                assert!(accounts.contains(&key3.account()));
                assert!(accounts.contains(&key4.account()));

                assert_eq!(scanner.completed, 1);
            }
        }
    }

    #[test]
    fn account_database_scanner() {
        const COUNT: usize = 4;

        // Prepare some accounts
        let mut lattice = UnsavedBlockLatticeBuilder::new();
        let mut blocks = Vec::new();
        let mut keys = Vec::new();
        {
            for _ in 0..COUNT {
                let key = PrivateKey::new();
                let send = lattice.genesis().send(&key, 1);
                let open = lattice.account(&key).receive(&send);
                blocks.push(send);
                blocks.push(open);
                keys.push(key);
            }
        }

        let ledger_ctx = LedgerContext::empty_dev();
        for mut block in blocks {
            let mut txn = ledger_ctx.ledger.rw_txn();
            ledger_ctx.ledger.process(&mut txn, &mut block).unwrap();
        }

        // Single batch
        {
            let mut scanner = AccountDatabaseIterator::new(ledger_ctx.ledger.clone());
            let tx = ledger_ctx.ledger.read_txn();
            let accounts = scanner.next_batch(&tx, 256);

            // Check that account set contains all keys
            assert_eq!(accounts.len(), keys.len() + 1); // +1 for genesis
            for key in &keys {
                assert!(accounts.contains(&key.account()));
            }
            assert_eq!(scanner.completed, 1);
        }

        // Multi batch
        {
            let mut scanner = AccountDatabaseIterator::new(ledger_ctx.ledger.clone());
            let tx = ledger_ctx.ledger.read_txn();

            // Request accounts in multiple batches
            let accounts1 = scanner.next_batch(&tx, 2);
            let accounts2 = scanner.next_batch(&tx, 2);
            let accounts3 = scanner.next_batch(&tx, 1);

            assert_eq!(accounts1.len(), 2);
            assert_eq!(accounts2.len(), 2);
            assert_eq!(accounts3.len(), 1);

            let mut accounts = accounts1;
            accounts.extend(accounts2);
            accounts.extend(accounts3);

            // Check that account set contains all keys
            for key in &keys {
                assert!(accounts.contains(&key.account()));
            }
            assert_eq!(scanner.completed, 1);
        }
    }
}
