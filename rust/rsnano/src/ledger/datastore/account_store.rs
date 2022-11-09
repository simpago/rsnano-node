use super::{iterator::DbIterator, ReadTransaction, Transaction, WriteTransaction};
use crate::core::{Account, AccountInfo};

pub type AccountIterator = Box<dyn DbIterator<Account, AccountInfo>>;

pub trait AccountStore {
    fn put(&self, transaction: &mut dyn WriteTransaction, account: &Account, info: &AccountInfo);
    fn get(&self, transaction: &dyn Transaction, account: &Account) -> Option<AccountInfo>;
    fn del(&self, transaction: &mut dyn WriteTransaction, account: &Account);
    fn begin_account(&self, transaction: &dyn Transaction, account: &Account) -> AccountIterator;
    fn begin(&self, transaction: &dyn Transaction) -> AccountIterator;
    fn for_each_par(
        &self,
        action: &(dyn Fn(&dyn ReadTransaction, AccountIterator, AccountIterator) + Send + Sync),
    );
    fn end(&self) -> AccountIterator;
    fn count(&self, txn: &dyn Transaction) -> usize;
    fn exists(&self, txn: &dyn Transaction, account: &Account) -> bool;
}
