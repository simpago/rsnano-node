use super::{as_write_txn, count, get, LmdbEnv, LmdbIteratorImpl};
use crate::{
    core::{
        deserialize_block_enum, Account, Amount, Block, BlockEnum, BlockHash, BlockSideband,
        BlockType, BlockVisitor, ChangeBlock, Epoch, OpenBlock, ReceiveBlock, SendBlock,
        StateBlock,
    },
    ledger::datastore::{
        block_store::BlockIterator, parallel_traversal, BlockStore, ReadTransaction, Transaction,
        WriteTransaction,
    },
    utils::{MemoryStream, Serialize, Stream, StreamAdapter},
};
use lmdb::{Database, DatabaseFlags, WriteFlags};
use num_traits::FromPrimitive;
use std::sync::Arc;

pub struct LmdbBlockStore {
    env: Arc<LmdbEnv>,
    database: Database,
}

impl LmdbBlockStore {
    pub fn new(env: Arc<LmdbEnv>) -> anyhow::Result<Self> {
        let database = env
            .environment
            .create_db(Some("blocks"), DatabaseFlags::empty())?;
        Ok(Self { env, database })
    }

    pub fn database(&self) -> Database {
        self.database
    }

    pub fn raw_put(&self, txn: &mut dyn WriteTransaction, data: &[u8], hash: &BlockHash) {
        as_write_txn(txn)
            .put(self.database, hash.as_bytes(), &data, WriteFlags::empty())
            .unwrap();
    }

    pub fn block_raw_get<'a>(
        &self,
        txn: &'a dyn Transaction,
        hash: &BlockHash,
    ) -> Option<&'a [u8]> {
        match get(txn, self.database, hash.as_bytes()) {
            Err(lmdb::Error::NotFound) => None,
            Ok(bytes) => Some(bytes),
            Err(e) => panic!("Could not load block. {:?}", e),
        }
    }
}

impl BlockStore for LmdbBlockStore {
    fn put(&self, txn: &mut dyn WriteTransaction, hash: &BlockHash, block: &dyn Block) {
        debug_assert!(
            block.sideband().unwrap().successor.is_zero()
                || self.exists(txn.txn(), &block.sideband().unwrap().successor)
        );

        let mut stream = MemoryStream::new();
        stream.write_u8(block.block_type() as u8).unwrap();
        block.serialize(&mut stream).unwrap();
        block
            .sideband()
            .unwrap()
            .serialize(&mut stream, block.block_type())
            .unwrap();
        self.raw_put(txn, stream.as_bytes(), hash);
        {
            let mut predecessor = BlockPredecessorMdbSet::new(txn, self);
            block.visit(&mut predecessor);
        }

        debug_assert!(
            block.previous().is_zero()
                || self
                    .successor(txn.txn(), &block.previous())
                    .unwrap_or_default()
                    == *hash
        );
    }

    fn exists(&self, transaction: &dyn Transaction, hash: &BlockHash) -> bool {
        self.block_raw_get(transaction, hash).is_some()
    }

    fn successor(&self, txn: &dyn Transaction, hash: &BlockHash) -> Option<BlockHash> {
        self.block_raw_get(txn, hash).map(|data| {
            debug_assert!(data.len() >= 32);
            let block_type = BlockType::from_u8(data[0]).unwrap();
            let offset = block_successor_offset(data.len(), block_type);
            BlockHash::from_bytes(data[offset..offset + 32].try_into().unwrap())
        })
    }

    fn successor_clear(&self, txn: &mut dyn WriteTransaction, hash: &BlockHash) {
        let value = self.block_raw_get(txn.txn(), hash).unwrap();
        let block_type = BlockType::from_u8(value[0]).unwrap();

        let mut data = value.to_vec();
        let offset = block_successor_offset(value.len(), block_type);
        data[offset..offset + BlockHash::serialized_size()].fill(0);
        self.raw_put(txn, &data, hash)
    }

    fn get(&self, txn: &dyn Transaction, hash: &BlockHash) -> Option<BlockEnum> {
        match self.block_raw_get(txn, hash) {
            None => None,
            Some(bytes) => {
                let mut stream = StreamAdapter::new(bytes);
                let mut block = deserialize_block_enum(&mut stream).unwrap();
                let sideband = BlockSideband::from_stream(&mut stream, block.block_type()).unwrap();
                block.as_block_mut().set_sideband(sideband);
                Some(block)
            }
        }
    }

    fn get_no_sideband(&self, txn: &dyn Transaction, hash: &BlockHash) -> Option<BlockEnum> {
        match self.block_raw_get(txn, hash) {
            None => None,
            Some(bytes) => {
                let mut stream = StreamAdapter::new(bytes);
                Some(deserialize_block_enum(&mut stream).unwrap())
            }
        }
    }

    fn del(&self, txn: &mut dyn WriteTransaction, hash: &BlockHash) {
        as_write_txn(txn)
            .del(self.database, hash.as_bytes(), None)
            .unwrap();
    }

    fn count(&self, txn: &dyn Transaction) -> usize {
        count(txn, self.database)
    }

    fn account_calculated(&self, block: &dyn Block) -> Account {
        let result = if block.account().is_zero() {
            block.sideband().unwrap().account
        } else {
            block.account()
        };

        debug_assert!(!result.is_zero());
        result
    }

    fn account(&self, txn: &dyn Transaction, hash: &BlockHash) -> Option<Account> {
        let block = self.get(txn, hash)?;
        Some(self.account_calculated(block.as_block()))
    }

    fn begin(&self, transaction: &dyn Transaction) -> BlockIterator {
        LmdbIteratorImpl::new_iterator(transaction, self.database, None, true)
    }

    fn begin_at_hash(&self, transaction: &dyn Transaction, hash: &BlockHash) -> BlockIterator {
        LmdbIteratorImpl::new_iterator(transaction, self.database, Some(hash.as_bytes()), true)
    }

    fn end(&self) -> BlockIterator {
        LmdbIteratorImpl::null_iterator()
    }

    fn random(&self, transaction: &dyn Transaction) -> Option<BlockEnum> {
        let hash = BlockHash::random();
        let mut existing = self.begin_at_hash(transaction, &hash);
        if existing.is_end() {
            existing = self.begin(transaction);
        }

        existing.current().map(|(_, v)| v.block.clone())
    }

    fn balance(&self, txn: &dyn Transaction, hash: &BlockHash) -> Amount {
        match self.get(txn, hash) {
            Some(block) => self.balance_calculated(&block),
            None => Amount::zero(),
        }
    }

    fn balance_calculated(&self, block: &BlockEnum) -> Amount {
        match block {
            BlockEnum::Send(b) => b.balance(),
            BlockEnum::Receive(b) => b.sideband().unwrap().balance,
            BlockEnum::Open(b) => b.sideband().unwrap().balance,
            BlockEnum::Change(b) => b.sideband().unwrap().balance,
            BlockEnum::State(b) => b.balance(),
        }
    }

    fn version(&self, txn: &dyn Transaction, hash: &BlockHash) -> Epoch {
        match self.get(txn, hash) {
            Some(block) => {
                if let BlockEnum::State(b) = block {
                    b.sideband().unwrap().details.epoch
                } else {
                    Epoch::Epoch0
                }
            }
            None => Epoch::Epoch0,
        }
    }

    fn for_each_par(
        &self,
        action: &(dyn Fn(&dyn ReadTransaction, BlockIterator, BlockIterator) + Send + Sync),
    ) {
        parallel_traversal(&|start, end, is_last| {
            let transaction = self.env.tx_begin_read().unwrap();
            let begin_it = self.begin_at_hash(&transaction, &start.into());
            let end_it = if !is_last {
                self.begin_at_hash(&transaction, &end.into())
            } else {
                self.end()
            };
            action(&transaction, begin_it, end_it);
        });
    }

    fn account_height(&self, txn: &dyn Transaction, hash: &BlockHash) -> u64 {
        match self.get(txn, hash) {
            Some(block) => block.as_block().sideband().unwrap().height,
            None => 0,
        }
    }
}

/// Fill in our predecessors
struct BlockPredecessorMdbSet<'a> {
    transaction: &'a mut dyn WriteTransaction,
    block_store: &'a LmdbBlockStore,
}

impl<'a, 'b> BlockPredecessorMdbSet<'a> {
    fn new(transaction: &'a mut dyn WriteTransaction, block_store: &'a LmdbBlockStore) -> Self {
        Self {
            transaction,
            block_store,
        }
    }

    fn fill_value(&mut self, block: &dyn Block) {
        let hash = block.hash();
        let t = self.transaction.txn();
        let value = self
            .block_store
            .block_raw_get(t, &block.previous())
            .unwrap();
        let mut data = value.to_vec();
        let block_type = BlockType::from_u8(data[0]).unwrap();

        let offset = block_successor_offset(data.len(), block_type);
        data[offset..offset + hash.as_bytes().len()].copy_from_slice(hash.as_bytes());

        self.block_store
            .raw_put(self.transaction, &data, &block.previous());
    }
}

impl<'a> BlockVisitor for BlockPredecessorMdbSet<'a> {
    fn send_block(&mut self, block: &SendBlock) {
        self.fill_value(block);
    }

    fn receive_block(&mut self, block: &ReceiveBlock) {
        self.fill_value(block);
    }

    fn open_block(&mut self, _block: &OpenBlock) {
        // Open blocks don't have a predecessor
    }

    fn change_block(&mut self, block: &ChangeBlock) {
        self.fill_value(block);
    }

    fn state_block(&mut self, block: &StateBlock) {
        if !block.previous().is_zero() {
            self.fill_value(block);
        }
    }
}

fn block_successor_offset(entry_size: usize, block_type: BlockType) -> usize {
    entry_size - BlockSideband::serialized_size(block_type)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::BlockBuilder, ledger::datastore::lmdb::TestLmdbEnv};

    #[test]
    fn empty() -> anyhow::Result<()> {
        let env = TestLmdbEnv::new();
        let store = LmdbBlockStore::new(env.env())?;
        let txn = env.tx_begin_read()?;
        assert!(store.get(&txn, &BlockHash::from(1)).is_none());
        assert_eq!(store.exists(&txn, &BlockHash::from(1)), false);
        assert_eq!(store.count(&txn), 0);
        Ok(())
    }

    #[test]
    fn add_block() -> anyhow::Result<()> {
        let env = TestLmdbEnv::new();
        let store = LmdbBlockStore::new(env.env())?;
        let mut txn = env.tx_begin_write()?;
        let block = BlockBuilder::open().build()?;
        let block_hash = block.hash();

        store.put(&mut txn, &block_hash, &block);
        let loaded = store.get(&txn, &block.hash()).expect("block not found");

        assert_eq!(loaded, BlockEnum::Open(block));
        assert!(store.exists(&txn, &block_hash));
        assert_eq!(store.count(&txn), 1);
        Ok(())
    }

    #[test]
    fn clear_successor() -> anyhow::Result<()> {
        let env = TestLmdbEnv::new();
        let store = LmdbBlockStore::new(env.env())?;
        let mut txn = env.tx_begin_write()?;

        let mut block1 = BlockBuilder::open()
            .account(Account::from(0))
            .representative(Account::from(1))
            .build()?;

        let block2 = BlockBuilder::open()
            .account(Account::from(0))
            .representative(Account::from(2))
            .build()?;

        let mut sideband = block1.sideband().unwrap().clone();
        sideband.successor = block2.hash();
        block1.set_sideband(sideband);

        store.put(&mut txn, &block2.hash(), &block2);
        store.put(&mut txn, &block1.hash(), &block1);

        store.successor_clear(&mut txn, &block1.hash());

        let loaded = store.get(&txn, &block1.hash()).expect("block not found");
        assert_eq!(
            loaded.as_block().sideband().unwrap().successor,
            BlockHash::zero()
        );
        Ok(())
    }

    #[test]
    fn add_two_blocks() -> anyhow::Result<()> {
        let env = TestLmdbEnv::new();
        let store = LmdbBlockStore::new(env.env())?;
        let mut txn = env.tx_begin_write()?;
        let block1 = BlockBuilder::open().build()?;
        let block2 = BlockBuilder::open().build()?;

        store.put(&mut txn, &block1.hash(), &block1);
        store.put(&mut txn, &block2.hash(), &block2);
        let loaded1 = store.get(&txn, &block1.hash()).expect("block1 not found");
        let loaded2 = store.get(&txn, &block2.hash()).expect("block2 not found");

        assert_eq!(loaded1, BlockEnum::Open(block1));
        assert_eq!(loaded2, BlockEnum::Open(block2));
        Ok(())
    }

    #[test]
    fn add_receive() -> anyhow::Result<()> {
        let env = TestLmdbEnv::new();
        let store = LmdbBlockStore::new(env.env())?;
        let block1 = BlockBuilder::open().build()?;
        let block2 = BlockBuilder::receive().previous(block1.hash()).build()?;
        let mut txn = env.tx_begin_write()?;
        store.put(&mut txn, &block1.hash(), &block1);
        store.put(&mut txn, &block2.hash(), &block2);
        let loaded = store.get(&txn, &block2.hash()).expect("block not found");
        assert_eq!(loaded, BlockEnum::Receive(block2));
        Ok(())
    }

    #[test]
    fn add_state() -> anyhow::Result<()> {
        let env = TestLmdbEnv::new();
        let store = LmdbBlockStore::new(env.env())?;
        let block1 = BlockBuilder::open().build()?;
        let block2 = BlockBuilder::state().previous(block1.hash()).build()?;
        let mut txn = env.tx_begin_write()?;
        store.put(&mut txn, &block1.hash(), &block1);
        store.put(&mut txn, &block2.hash(), &block2);
        let loaded = store.get(&txn, &block2.hash()).expect("block not found");
        assert_eq!(loaded, BlockEnum::State(block2));
        Ok(())
    }

    #[test]
    fn replace_block() -> anyhow::Result<()> {
        let env = TestLmdbEnv::new();
        let store = LmdbBlockStore::new(env.env())?;
        let mut txn = env.tx_begin_write()?;
        let open = BlockBuilder::open().build()?;
        let send1 = BlockBuilder::send().previous(open.hash()).build()?;
        let mut send2 = send1.clone();
        send2.set_work(12345);

        store.put(&mut txn, &open.hash(), &open);
        store.put(&mut txn, &send1.hash(), &send1);
        store.put(&mut txn, &send2.hash(), &send2);

        assert_eq!(store.count(&txn), 2);
        assert_eq!(
            store.get(&txn, &send1.hash()).unwrap().as_block().work(),
            12345
        );
        Ok(())
    }

    #[test]
    fn random() -> anyhow::Result<()> {
        let env = TestLmdbEnv::new();
        let store = LmdbBlockStore::new(env.env())?;
        let mut txn = env.tx_begin_write()?;
        let block = BlockBuilder::open().build()?;
        let block_hash = block.hash();

        store.put(&mut txn, &block_hash, &block);
        let random = store.random(&txn).expect("block not found");

        assert_eq!(random, BlockEnum::Open(block));
        Ok(())
    }

    #[test]
    fn reset_renew_existing_transaction() -> anyhow::Result<()> {
        let env = TestLmdbEnv::new();
        let store = LmdbBlockStore::new(env.env())?;
        let block = BlockBuilder::open().build()?;
        let block_hash = block.hash();

        let mut read_txn = env.tx_begin_read()?;
        read_txn.reset();
        {
            let mut txn = env.tx_begin_write()?;
            store.put(&mut txn, &block_hash, &block);
        }
        read_txn.renew();
        assert!(store.exists(&read_txn, &block_hash));
        Ok(())
    }
}
