use crate::{Account, BlockHash};

use super::{DbIterator, ReadTransaction, Transaction, WriteTransaction};

/// Maps head block to owning account
/// BlockHash -> Account
pub trait FrontierStore {
    fn put(&self, txn: &dyn WriteTransaction, hash: &BlockHash, account: &Account);
    fn get(&self, txn: &dyn Transaction, hash: &BlockHash) -> Account;
    fn del(&self, txn: &dyn WriteTransaction, hash: &BlockHash);
    fn begin(&self, txn: &dyn Transaction) -> Box<dyn DbIterator<BlockHash, Account>>;

    fn begin_at_hash(
        &self,
        txn: &dyn Transaction,
        hash: &BlockHash,
    ) -> Box<dyn DbIterator<BlockHash, Account>>;

    fn for_each_par(
        &self,
        action: &(dyn Fn(
            &dyn ReadTransaction,
            &mut dyn DbIterator<BlockHash, Account>,
            &mut dyn DbIterator<BlockHash, Account>,
        ) + Send
              + Sync),
    );

    fn end(&self) -> Box<dyn DbIterator<BlockHash, Account>>;
}
