use crate::Amount;

use super::{DbIterator, Transaction, WriteTransaction};

/// Samples of online vote weight
/// u64 -> Amount
pub trait OnlineWeightStore {
    fn put(&self, txn: &dyn WriteTransaction, time: u64, amount: &Amount);
    fn del(&self, txn: &dyn WriteTransaction, time: u64);
    fn begin(&self, txn: &dyn Transaction) -> Box<dyn DbIterator<u64, Amount>>;
    fn rbegin(&self, txn: &dyn Transaction) -> Box<dyn DbIterator<u64, Amount>>;
    fn count(&self, txn: &dyn Transaction) -> usize;
    fn clear(&self, txn: &dyn WriteTransaction);
}
