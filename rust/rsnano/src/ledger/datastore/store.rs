use std::path::Path;

use super::{
    AccountStore, BlockStore, ConfirmationHeightStore, FrontierStore, PendingStore, PrunedStore,
    ReadTransaction, WriteTransaction,
};

pub trait Store: Send + Sync {
    fn tx_begin_read(&self) -> anyhow::Result<Box<dyn ReadTransaction>>;
    fn tx_begin_write(&self) -> anyhow::Result<Box<dyn WriteTransaction>>;
    fn copy_db(&self, destination: &Path) -> anyhow::Result<()>;
    fn account(&self) -> &dyn AccountStore;
    fn confirmation_height(&self) -> &dyn ConfirmationHeightStore;
    fn pruned(&self) -> &dyn PrunedStore;
    fn block(&self) -> &dyn BlockStore;
    fn pending(&self) -> &dyn PendingStore;
    fn frontier(&self) -> &dyn FrontierStore;
}
