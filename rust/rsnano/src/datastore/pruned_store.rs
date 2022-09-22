use super::{DbIterator, ReadTransaction, Transaction, WriteTransaction};
use crate::{BlockHash, NoValue};

/// Pruned blocks hashes
pub trait PrunedStore {
    fn put(&self, txn: &dyn WriteTransaction, hash: &BlockHash);
    fn del(&self, txn: &dyn WriteTransaction, hash: &BlockHash);
    fn exists(&self, txn: &dyn Transaction, hash: &BlockHash) -> bool;
    fn begin(&self, txn: &dyn Transaction) -> Box<dyn DbIterator<BlockHash, NoValue>>;

    fn begin_at_hash(
        &self,
        txn: &dyn Transaction,
        hash: &BlockHash,
    ) -> Box<dyn DbIterator<BlockHash, NoValue>>;

    fn end(&self) -> Box<dyn DbIterator<BlockHash, NoValue>>;

    fn random(&self, txn: &dyn Transaction) -> BlockHash;
    fn count(&self, txn: &dyn Transaction) -> usize;
    fn clear(&self, txn: &dyn WriteTransaction);
    fn for_each_par(
        &self,
        action: &(dyn Fn(
            &dyn ReadTransaction,
            &mut dyn DbIterator<BlockHash, NoValue>,
            &mut dyn DbIterator<BlockHash, NoValue>,
        ) + Send
              + Sync),
    );
}
