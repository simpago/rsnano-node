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
        let mut it = self.ledger.store.pending.begin_at_key(tx, &self.next);
        // TODO: This pending iteration heuristic should be encapsulated in a pending_iterator class and reused across other components
        // The heuristic is to advance the iterator sequentially until we reach a new account or perform a fresh lookup if the account has too many pending blocks
        // This is to avoid the overhead of performing a fresh lookup for every pending account as majority of accounts have only a few pending blocks

        let mut count = 0;
        let mut end_reached = true;
        while let Some((key, _)) = it.current() {
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

            for _ in 0..SEQUENTIAL_ATTEMPTS {
                let current = it.current();
                let Some(current) = &current else {
                    break;
                };
                if current.0.receiving_account != starting_account {
                    break;
                }
                it.next();
            }

            // If we didn't advance to the next account, perform a fresh lookup
            if let Some(current) = it.current() {
                if current.0.receiving_account == starting_account {
                    it = self.ledger.store.pending.begin_at_key(
                        tx,
                        &PendingKey::new(
                            starting_account.inc().unwrap_or_default(),
                            BlockHash::zero(),
                        ),
                    );
                }
            }
            debug_assert!(
                it.is_end() || it.current().unwrap().0.receiving_account != starting_account
            );
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
    use rsnano_core::{
        work::{WorkPool, STUB_WORK_POOL},
        Amount, Block, PrivateKey, StateBlock, DEV_GENESIS_KEY,
    };
    use rsnano_ledger::{LedgerContext, DEV_GENESIS_HASH};

    #[test]
    fn pending_database_scanner() {
        // Prepare pending sends from genesis
        // 1 account with 1 pending
        // 1 account with 21 pendings
        // 2 accounts with 1 pending each
        let mut blocks = Vec::new();
        let key1 = PrivateKey::new();
        let key2 = PrivateKey::new();
        let key3 = PrivateKey::new();
        let key4 = PrivateKey::new();
        {
            let source = &DEV_GENESIS_KEY;
            let mut latest = *DEV_GENESIS_HASH;
            let mut balance = Amount::MAX;

            // 1 account with 1 pending
            {
                let send = Block::State(StateBlock::new(
                    source.account(),
                    latest,
                    source.public_key(),
                    balance - Amount::raw(1),
                    key1.account().into(),
                    source,
                    STUB_WORK_POOL.generate_dev2(latest.into()).unwrap(),
                ));
                latest = send.hash();
                balance = send.balance_field().unwrap();
                blocks.push(send);
            }
            // 1 account with 21 pendings
            for _ in 0..21 {
                let send = Block::State(StateBlock::new(
                    source.account(),
                    latest,
                    source.public_key(),
                    balance - Amount::raw(1),
                    key2.account().into(),
                    source,
                    STUB_WORK_POOL.generate_dev2(latest.into()).unwrap(),
                ));
                latest = send.hash();
                balance = send.balance_field().unwrap();
                blocks.push(send);
            }
            // 2 accounts with 1 pending each
            {
                let send = Block::State(StateBlock::new(
                    source.account(),
                    latest,
                    source.public_key(),
                    balance - Amount::raw(1),
                    key3.account().into(),
                    source,
                    STUB_WORK_POOL.generate_dev2(latest.into()).unwrap(),
                ));
                latest = send.hash();
                balance = send.balance_field().unwrap();
                blocks.push(send);
            }
            {
                let send = Block::State(StateBlock::new(
                    source.account(),
                    latest,
                    source.public_key(),
                    balance - Amount::raw(1),
                    key4.account().into(),
                    source,
                    STUB_WORK_POOL.generate_dev2(latest.into()).unwrap(),
                ));
                blocks.push(send);
            }

            let ledger_ctx = LedgerContext::empty();
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
        let mut blocks = Vec::new();
        let mut keys = Vec::new();
        {
            let source = &DEV_GENESIS_KEY;
            let mut latest = *DEV_GENESIS_HASH;
            let mut balance = Amount::MAX;

            for _ in 0..COUNT {
                let key = PrivateKey::new();
                let send = Block::State(StateBlock::new(
                    source.account(),
                    latest,
                    source.public_key(),
                    balance - Amount::raw(1),
                    key.account().into(),
                    source,
                    STUB_WORK_POOL.generate_dev2(latest.into()).unwrap(),
                ));
                let open = Block::State(StateBlock::new(
                    key.account(),
                    BlockHash::zero(),
                    key.public_key(),
                    Amount::raw(1),
                    send.hash().into(),
                    &key,
                    STUB_WORK_POOL.generate_dev2(key.account().into()).unwrap(),
                ));
                latest = send.hash();
                balance = send.balance_field().unwrap();
                blocks.push(send);
                blocks.push(open);
                keys.push(key);
            }
        }

        let ledger_ctx = LedgerContext::empty();
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
