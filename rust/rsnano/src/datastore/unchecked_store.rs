use crate::{
    unchecked_info::{UncheckedInfo, UncheckedKey},
    HashOrAccount,
};

use super::{DbIterator, Transaction, WriteTransaction};

/// Unchecked bootstrap blocks info.
/// BlockHash -> UncheckedInfo
pub trait UncheckedStore {
    fn clear(&self, txn: &dyn WriteTransaction);
    fn put(&self, txn: &dyn WriteTransaction, dependency: &HashOrAccount, info: &UncheckedInfo);
    fn exists(&self, txn: &dyn Transaction, key: &UncheckedKey) -> bool;
    fn del(&self, txn: &dyn WriteTransaction, key: &UncheckedKey);
    fn begin(&self, txn: &dyn Transaction) -> Box<dyn DbIterator<UncheckedKey, UncheckedInfo>>;
    fn lower_bound(
        &self,
        txn: &dyn Transaction,
        key: &UncheckedKey,
    ) -> Box<dyn DbIterator<UncheckedKey, UncheckedInfo>>;
    fn count(&self, txn: &dyn Transaction) -> usize;
}
