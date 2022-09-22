use super::{DbIterator, ReadTransaction, Transaction, WriteTransaction};
use crate::{BlockHash, QualifiedRoot, Root};

pub trait FinalVoteStore {
    fn put(&self, txn: &dyn WriteTransaction, root: &QualifiedRoot, hash: &BlockHash) -> bool;
    fn begin(&self, txn: &dyn Transaction) -> Box<dyn DbIterator<QualifiedRoot, BlockHash>>;
    fn begin_at_root(
        &self,
        txn: &dyn Transaction,
        root: &QualifiedRoot,
    ) -> Box<dyn DbIterator<QualifiedRoot, BlockHash>>;
    fn end(&self) -> Box<dyn DbIterator<QualifiedRoot, BlockHash>>;
    fn get(&self, txn: &dyn Transaction, root: Root) -> Vec<BlockHash>;
    fn del(&self, txn: &dyn WriteTransaction, root: Root);
    fn count(&self, txn: &dyn Transaction) -> usize;
    fn clear(&self, txn: &dyn WriteTransaction);
    fn for_each_par(
        &self,
        action: &(dyn Fn(
            &dyn ReadTransaction,
            &mut dyn DbIterator<QualifiedRoot, BlockHash>,
            &mut dyn DbIterator<QualifiedRoot, BlockHash>,
        ) + Send
              + Sync),
    );
}
