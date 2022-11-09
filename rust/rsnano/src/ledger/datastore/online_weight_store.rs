use crate::core::Amount;

use super::{DbIterator, Transaction, WriteTransaction};

pub type OnlineWeightIterator = Box<dyn DbIterator<u64, Amount>>;

/// Samples of online vote weight
/// u64 -> Amount
pub trait OnlineWeightStore {
    fn put(&self, txn: &mut dyn WriteTransaction, time: u64, amount: &Amount);
    fn del(&self, txn: &mut dyn WriteTransaction, time: u64);
    fn begin(&self, txn: &dyn Transaction) -> OnlineWeightIterator;
    fn rbegin(&self, txn: &dyn Transaction) -> OnlineWeightIterator;
    fn count(&self, txn: &dyn Transaction) -> usize;
    fn clear(&self, txn: &mut dyn WriteTransaction);
}
