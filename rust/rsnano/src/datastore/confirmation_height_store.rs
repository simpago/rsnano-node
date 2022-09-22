use super::{DbIterator, ReadTransaction, Transaction, WriteTransaction};
use crate::{Account, ConfirmationHeightInfo};

pub trait ConfirmationHeightStore {
    fn put(&self, txn: &dyn WriteTransaction, account: &Account, info: &ConfirmationHeightInfo);
    fn get(&self, txn: &dyn Transaction, account: &Account) -> Option<ConfirmationHeightInfo>;
    fn exists(&self, txn: &dyn Transaction, account: &Account) -> bool;
    fn del(&self, txn: &dyn Transaction, account: &Account);
    fn count(&self, txn: &dyn Transaction) -> usize;
    fn clear(&self, txn: &dyn WriteTransaction);
    fn begin(&self, txn: &dyn Transaction) -> Box<dyn DbIterator<Account, ConfirmationHeightInfo>>;
    fn begin_at_account(
        &self,
        txn: &dyn Transaction,
        account: &Account,
    ) -> Box<dyn DbIterator<Account, ConfirmationHeightInfo>>;
    fn end(&self) -> Box<dyn DbIterator<Account, ConfirmationHeightInfo>>;
    fn for_each_par(
        &self,
        action: &(dyn Fn(
            &dyn ReadTransaction,
            &mut dyn DbIterator<Account, ConfirmationHeightInfo>,
            &mut dyn DbIterator<Account, ConfirmationHeightInfo>,
        ) + Send
              + Sync),
    );
}
