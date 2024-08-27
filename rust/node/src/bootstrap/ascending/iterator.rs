use std::{collections::VecDeque, sync::Arc};

use rsnano_core::Account;
use rsnano_ledger::Ledger;
use rsnano_store_lmdb::LmdbReadTransaction;

enum TableType {
    Account,
    Pending,
}

struct DatabaseIterator {
    ledger: Arc<Ledger>,
    current: Account,
    table: TableType,
}

impl DatabaseIterator {
    pub fn new(ledger: Arc<Ledger>, table: TableType) -> Self {
        Self {
            ledger,
            current: Account::zero(),
            table,
        }
    }

    pub fn next(&mut self, tx: &LmdbReadTransaction) -> Account {
        match self.table {
            TableType::Account => {
                let i = self.current.inc().unwrap_or(Account::zero());
                if let Some((account, _)) = self.ledger.any().accounts_range(tx, i..).next() {
                    self.current = account;
                } else {
                    self.current = Account::zero();
                }
            }
            TableType::Pending => {
                if let Some((key, _)) = self
                    .ledger
                    .any()
                    .receivable_upper_bound(tx, self.current)
                    .next()
                {
                    self.current = key.receiving_account;
                } else {
                    self.current = Account::zero();
                }
            }
        }

        self.current
    }
}

pub(crate) struct BufferedIterator {
    ledger: Arc<Ledger>,
    buffer: VecDeque<Account>,
    warmup: bool,
    accounts_iterator: DatabaseIterator,
    pending_iterator: DatabaseIterator,
}

impl BufferedIterator {
    const SIZE: usize = 1024;

    pub fn new(ledger: Arc<Ledger>) -> Self {
        Self {
            buffer: VecDeque::new(),
            warmup: true,
            accounts_iterator: DatabaseIterator::new(Arc::clone(&ledger), TableType::Account),
            pending_iterator: DatabaseIterator::new(Arc::clone(&ledger), TableType::Pending),
            ledger,
        }
    }

    pub fn next(&mut self, filter: impl Fn(&Account) -> bool) -> Account {
        if self.buffer.is_empty() {
            self.fill();
        }

        while let Some(result) = self.buffer.pop_front() {
            if filter(&result) {
                return result;
            }
        }

        Account::zero()
    }

    pub fn warmup(&self) -> bool {
        self.warmup
    }

    fn fill(&mut self) {
        debug_assert!(self.buffer.is_empty());

        // Fill half from accounts table and half from pending table
        let tx = self.ledger.read_txn();

        for _ in 0..Self::SIZE / 2 {
            let account = self.accounts_iterator.next(&tx);
            if !account.is_zero() {
                self.buffer.push_back(account);
            }
        }

        for _ in 0..Self::SIZE / 2 {
            let account = self.pending_iterator.next(&tx);
            if !account.is_zero() {
                self.buffer.push_back(account);
            } else {
                self.warmup = false;
            }
        }
    }
}
