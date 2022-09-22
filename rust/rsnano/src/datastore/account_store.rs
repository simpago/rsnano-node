use super::{DbIterator, ReadTransaction, Transaction, WriteTransaction};
use crate::{Account, AccountInfo};

pub trait AccountStore {
    fn put(&self, transaction: &dyn WriteTransaction, account: &Account, info: &AccountInfo);
    fn get(&self, transaction: &dyn Transaction, account: &Account) -> Option<AccountInfo>;
    fn del(&self, transaction: &dyn WriteTransaction, account: &Account);
    fn begin_account(
        &self,
        transaction: &dyn Transaction,
        account: &Account,
    ) -> Box<dyn DbIterator<Account, AccountInfo>>;
    fn begin(&self, transaction: &dyn Transaction) -> Box<dyn DbIterator<Account, AccountInfo>>;
    fn rbegin(&self, transaction: &dyn Transaction) -> Box<dyn DbIterator<Account, AccountInfo>>;
    fn for_each_par(
        &self,
        action: &(dyn Fn(
            &dyn ReadTransaction,
            &mut dyn DbIterator<Account, AccountInfo>,
            &mut dyn DbIterator<Account, AccountInfo>,
        ) + Send
              + Sync),
    );
    fn end(&self) -> Box<dyn DbIterator<Account, AccountInfo>>;
    fn count(&self, txn: &dyn Transaction) -> usize;
}
