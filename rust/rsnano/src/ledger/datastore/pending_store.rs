use super::{DbIterator, ReadTransaction, Transaction, WriteTransaction};
use crate::core::{Account, PendingInfo, PendingKey};

pub type PendingIterator = Box<dyn DbIterator<PendingKey, PendingInfo>>;

/// Maps (destination account, pending block) to (source account, amount, version).
/// nano::account, nano::block_hash -> nano::account, nano::amount, nano::epoch
pub trait PendingStore {
    fn put(&self, txn: &mut dyn WriteTransaction, key: &PendingKey, pending: &PendingInfo);
    fn del(&self, txn: &mut dyn WriteTransaction, key: &PendingKey);
    fn get(&self, txn: &dyn Transaction, key: &PendingKey) -> Option<PendingInfo>;
    fn begin(&self, txn: &dyn Transaction) -> PendingIterator;
    fn begin_at_key(&self, txn: &dyn Transaction, key: &PendingKey) -> PendingIterator;
    fn end(&self) -> PendingIterator;
    fn exists(&self, txn: &dyn Transaction, key: &PendingKey) -> bool;
    fn any(&self, txn: &dyn Transaction, account: &Account) -> bool;
    fn for_each_par(
        &self,
        action: &(dyn Fn(&dyn ReadTransaction, PendingIterator, PendingIterator) + Send + Sync),
    );
}
