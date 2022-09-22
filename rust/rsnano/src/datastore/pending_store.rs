use crate::{Account, PendingInfo, PendingKey};

use super::{DbIterator, ReadTransaction, Transaction, WriteTransaction};

/// Maps (destination account, pending block) to (source account, amount, version).
/// nano::account, nano::block_hash -> nano::account, nano::amount, nano::epoch
pub trait PendingStore {
    fn put(&self, txn: &dyn WriteTransaction, key: &PendingKey, pending: &PendingInfo);
    fn del(&self, txn: &dyn WriteTransaction, key: &PendingKey);
    fn get(&self, txn: &dyn Transaction, key: &PendingKey) -> Option<PendingInfo>;
    fn begin(&self, txn: &dyn Transaction) -> Box<dyn DbIterator<PendingKey, PendingInfo>>;
    fn begin_at_key(
        &self,
        txn: &dyn Transaction,
        key: &PendingKey,
    ) -> Box<dyn DbIterator<PendingKey, PendingInfo>>;
    fn end(&self) -> Box<dyn DbIterator<PendingKey, PendingInfo>>;
    fn exists(&self, txn: &dyn Transaction, key: &PendingKey) -> bool;
    fn any(&self, txn: &dyn Transaction, account: &Account) -> bool;
    fn for_each_par(
        &self,
        action: &(dyn Fn(
            &dyn ReadTransaction,
            &mut dyn DbIterator<PendingKey, PendingInfo>,
            &mut dyn DbIterator<PendingKey, PendingInfo>,
        ) + Send
              + Sync),
    );
}
