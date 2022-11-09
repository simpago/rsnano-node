use super::{DbIterator, ReadTransaction, Transaction, WriteTransaction};
use crate::core::{Account, ConfirmationHeightInfo};

pub type ConfirmationHeightIterator = Box<dyn DbIterator<Account, ConfirmationHeightInfo>>;

pub trait ConfirmationHeightStore {
    fn put(&self, txn: &mut dyn WriteTransaction, account: &Account, info: &ConfirmationHeightInfo);
    fn get(&self, txn: &dyn Transaction, account: &Account) -> Option<ConfirmationHeightInfo>;
    fn exists(&self, txn: &dyn Transaction, account: &Account) -> bool;
    fn del(&self, txn: &mut dyn WriteTransaction, account: &Account);
    fn count(&self, txn: &dyn Transaction) -> usize;
    fn clear(&self, txn: &mut dyn WriteTransaction);
    fn begin(&self, txn: &dyn Transaction) -> ConfirmationHeightIterator;
    fn begin_at_account(
        &self,
        txn: &dyn Transaction,
        account: &Account,
    ) -> ConfirmationHeightIterator;
    fn end(&self) -> ConfirmationHeightIterator;
    fn for_each_par(
        &self,
        action: &(dyn Fn(&dyn ReadTransaction, ConfirmationHeightIterator, ConfirmationHeightIterator)
              + Send
              + Sync),
    );
}
