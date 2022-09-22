use crate::{Account, Amount, Block, BlockEnum, BlockHash, BlockWithSideband, Epoch};

use super::{DbIterator, ReadTransaction, Transaction, WriteTransaction};

pub trait BlockStore {
    fn put(&self, txn: &dyn WriteTransaction, hash: &BlockHash, block: &dyn Block);
    fn exists(&self, txn: &dyn Transaction, hash: &BlockHash) -> bool;
    fn successor(&self, txn: &dyn Transaction, hash: &BlockHash) -> BlockHash;
    fn successor_clear(&self, txn: &dyn WriteTransaction, hash: &BlockHash);
    fn get(&self, txn: &dyn Transaction, hash: &BlockHash) -> Option<BlockEnum>;
    fn get_no_sideband(&self, txn: &dyn Transaction, hash: &BlockHash) -> Option<BlockEnum>;
    fn del(&self, txn: &dyn WriteTransaction, hash: &BlockHash);
    fn count(&self, txn: &dyn Transaction) -> usize;
    fn account_calculated(&self, block: &dyn Block) -> Account;
    fn account(&self, txn: &dyn Transaction, hash: &BlockHash) -> Account;
    fn begin(&self, txn: &dyn Transaction) -> Box<dyn DbIterator<BlockHash, BlockWithSideband>>;
    fn begin_at_hash(
        &self,
        txn: &dyn Transaction,
        hash: &BlockHash,
    ) -> Box<dyn DbIterator<BlockHash, BlockWithSideband>>;
    fn end(&self) -> Box<dyn DbIterator<BlockHash, BlockWithSideband>>;
    fn random(&self, txn: &dyn Transaction) -> Option<BlockEnum>;
    fn balance(&self, txn: &dyn Transaction, hash: &BlockHash) -> Amount;
    fn balance_calculated(&self, block: &BlockEnum) -> Amount;
    fn version(&self, txn: &dyn Transaction, hash: &BlockHash) -> Epoch;
    fn for_each_par(
        &self,
        action: &(dyn Fn(
            &dyn ReadTransaction,
            &mut dyn DbIterator<BlockHash, BlockWithSideband>,
            &mut dyn DbIterator<BlockHash, BlockWithSideband>,
        ) + Send
              + Sync),
    );
    fn account_height(&self, txn: &dyn Transaction, hash: &BlockHash) -> u64;
}
