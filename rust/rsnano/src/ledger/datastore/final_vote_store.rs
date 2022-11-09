use super::{DbIterator, ReadTransaction, Transaction, WriteTransaction};
use crate::core::{BlockHash, QualifiedRoot, Root};

pub type FinalVoteIterator = Box<dyn DbIterator<QualifiedRoot, BlockHash>>;

pub trait FinalVoteStore {
    fn put(&self, txn: &mut dyn WriteTransaction, root: &QualifiedRoot, hash: &BlockHash) -> bool;
    fn begin(&self, txn: &dyn Transaction) -> FinalVoteIterator;
    fn begin_at_root(&self, txn: &dyn Transaction, root: &QualifiedRoot) -> FinalVoteIterator;
    fn end(&self) -> FinalVoteIterator;
    fn get(&self, txn: &dyn Transaction, root: Root) -> Vec<BlockHash>;
    fn del(&self, txn: &mut dyn WriteTransaction, root: &Root);
    fn count(&self, txn: &dyn Transaction) -> usize;
    fn clear(&self, txn: &mut dyn WriteTransaction);
    fn for_each_par(
        &self,
        action: &(dyn Fn(&dyn ReadTransaction, FinalVoteIterator, FinalVoteIterator) + Send + Sync),
    );
}
