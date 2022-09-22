use crate::{
    datastore::{
        lmdb::{MDB_NOTFOUND, MDB_SUCCESS},
        parallel_traversal, BlockStore, DbIterator, NullIterator, ReadTransaction, Transaction,
        WriteTransaction,
    },
    deserialize_block_enum,
    utils::{MemoryStream, Serialize, Stream, StreamAdapter},
    Account, Amount, Block, BlockEnum, BlockHash, BlockSideband, BlockType, BlockVisitor,
    BlockWithSideband, Epoch,
};
use num_traits::FromPrimitive;
use std::{
    ffi::c_void,
    sync::{Arc, Mutex},
};

use super::{
    assert_success, ensure_success, get_raw_lmdb_txn, mdb_count, mdb_dbi_open, mdb_del, mdb_get,
    mdb_put, LmdbEnv, LmdbIterator, LmdbWriteTransaction, MdbVal,
};

pub struct LmdbBlockStore {
    env: Arc<LmdbEnv>,
    db_handle: Mutex<u32>,
}

impl LmdbBlockStore {
    pub fn new(env: Arc<LmdbEnv>) -> Self {
        Self {
            env,
            db_handle: Mutex::new(0),
        }
    }

    pub fn db_handle(&self) -> u32 {
        *self.db_handle.lock().unwrap()
    }

    pub fn raw_put(&self, txn: &LmdbWriteTransaction, data: &[u8], hash: &BlockHash) {
        let mut key = MdbVal::from_slice(hash.as_bytes());
        let mut data = MdbVal::from_slice(data);
        let status = unsafe { mdb_put(txn.handle, self.db_handle(), &mut key, &mut data, 0) };
        assert_success(status);
    }

    pub fn block_raw_get(&self, txn: &dyn Transaction, hash: &BlockHash, value: &mut MdbVal) {
        let mut key = MdbVal::from_slice(hash.as_bytes());
        let status = unsafe { mdb_get(get_raw_lmdb_txn(txn), self.db_handle(), &mut key, value) };
        assert!(status == MDB_SUCCESS || status == MDB_NOTFOUND);
    }

    pub fn open_db(&self, txn: &dyn Transaction, flags: u32) -> anyhow::Result<()> {
        let mut handle = 0;
        let status = unsafe { mdb_dbi_open(get_raw_lmdb_txn(txn), "blocks", flags, &mut handle) };
        let mut guard = self.db_handle.lock().unwrap();
        *guard = handle;
        ensure_success(status)
    }
}

unsafe fn block_type_from_raw(data: *const c_void) -> Option<BlockType> {
    // The block type is the first byte
    let first_byte = *(data as *const u8);
    BlockType::from_u8(first_byte)
}

impl BlockStore for LmdbBlockStore {
    fn put(&self, txn: &dyn WriteTransaction, hash: &BlockHash, block: &dyn Block) {
        debug_assert!(
            block.sideband().unwrap().successor.is_zero()
                || self.exists(txn.as_transaction(), &block.sideband().unwrap().successor)
        );

        let lmdb_txn = txn.as_any().downcast_ref::<LmdbWriteTransaction>().unwrap();
        let mut stream = MemoryStream::new();
        stream.write_u8(block.block_type() as u8).unwrap();
        block.serialize(&mut stream).unwrap();
        block
            .sideband()
            .unwrap()
            .serialize(&mut stream, block.block_type())
            .unwrap();
        self.raw_put(lmdb_txn, stream.as_bytes(), hash);
        let mut predecessor = BlockPredecessorMdbSet::new(lmdb_txn, self);
        block.visit(&mut predecessor);

        debug_assert!(
            block.previous().is_zero()
                || self.successor(txn.as_transaction(), block.previous()) == *hash
        );
    }

    fn exists(&self, transaction: &dyn Transaction, hash: &BlockHash) -> bool {
        let mut junk = MdbVal::new();
        self.block_raw_get(transaction, hash, &mut junk);
        junk.mv_size != 0
    }

    fn successor(&self, txn: &dyn Transaction, hash: &BlockHash) -> BlockHash {
        let mut value = MdbVal::new();
        self.block_raw_get(txn, hash, &mut value);
        let data = value.as_slice();
        if data.len() != 0 {
            debug_assert!(data.len() >= 32);
            let block_type = BlockType::from_u8(data[0]).unwrap();
            let offset = block_successor_offset(data.len(), block_type);
            BlockHash::from_bytes(data[offset..offset + 32].try_into().unwrap())
        } else {
            BlockHash::new()
        }
    }

    fn successor_clear(&self, txn: &dyn WriteTransaction, hash: &BlockHash) {
        let mut value = MdbVal::new();
        self.block_raw_get(txn.as_transaction(), hash, &mut value);
        debug_assert!(value.mv_size != 0);
        let block_type = unsafe { block_type_from_raw(value.mv_data) }.unwrap();

        let mut data = value.as_slice().to_vec();
        let offset = block_successor_offset(value.mv_size, block_type);
        data[offset..offset + BlockHash::serialized_size()].fill(0);
        let lmdb_txn = txn.as_any().downcast_ref::<LmdbWriteTransaction>().unwrap();
        self.raw_put(lmdb_txn, &data, hash)
    }

    fn get(&self, txn: &dyn Transaction, hash: &BlockHash) -> Option<BlockEnum> {
        let mut value = MdbVal::new();
        self.block_raw_get(txn, hash, &mut value);
        if value.mv_size != 0 {
            let mut stream = StreamAdapter::new(value.as_slice());
            let mut block = deserialize_block_enum(&mut stream).unwrap();
            let sideband = BlockSideband::from_stream(&mut stream, block.block_type()).unwrap();
            block.as_block_mut().set_sideband(sideband);
            Some(block)
        } else {
            None
        }
    }

    fn get_no_sideband(&self, txn: &dyn Transaction, hash: &BlockHash) -> Option<BlockEnum> {
        let mut value = MdbVal::new();
        self.block_raw_get(txn, hash, &mut value);
        if value.mv_size != 0 {
            let mut stream = StreamAdapter::new(value.as_slice());
            Some(deserialize_block_enum(&mut stream).unwrap())
        } else {
            None
        }
    }

    fn del(&self, txn: &dyn WriteTransaction, hash: &BlockHash) {
        let txn = txn.as_any().downcast_ref::<LmdbWriteTransaction>().unwrap();
        let status = unsafe {
            mdb_del(
                txn.handle,
                self.db_handle(),
                &mut MdbVal::from_slice(hash.as_bytes()),
                None,
            )
        };
        assert_success(status);
    }

    fn count(&self, txn: &dyn Transaction) -> usize {
        unsafe { mdb_count(get_raw_lmdb_txn(txn), self.db_handle()) }
    }

    fn account_calculated(&self, block: &dyn Block) -> Account {
        let result = if block.account().is_zero() {
            block.sideband().unwrap().account
        } else {
            *block.account()
        };

        debug_assert!(!result.is_zero());
        result
    }

    fn account(&self, txn: &dyn Transaction, hash: &BlockHash) -> Account {
        let block = self.get(txn, hash).unwrap();
        self.account_calculated(block.as_block())
    }

    fn begin(
        &self,
        transaction: &dyn Transaction,
    ) -> Box<dyn DbIterator<BlockHash, BlockWithSideband>> {
        Box::new(LmdbIterator::new(transaction, self.db_handle(), None, true))
    }

    fn begin_at_hash(
        &self,
        transaction: &dyn Transaction,
        hash: &BlockHash,
    ) -> Box<dyn DbIterator<BlockHash, BlockWithSideband>> {
        Box::new(LmdbIterator::new(
            transaction,
            self.db_handle(),
            Some(hash),
            true,
        ))
    }

    fn end(&self) -> Box<dyn DbIterator<BlockHash, BlockWithSideband>> {
        Box::new(NullIterator::new())
    }

    fn random(&self, transaction: &dyn Transaction) -> Option<BlockEnum> {
        let hash = BlockHash::random();
        let mut existing = self.begin_at_hash(transaction, &hash);
        if existing.is_end() {
            existing = self.begin(transaction);
        }

        existing.value().map(|i| i.block.clone())
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

    fn version(&self, txn: &dyn Transaction, hash: &BlockHash) -> crate::Epoch {
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
        action: &(dyn Fn(
            &dyn ReadTransaction,
            &mut dyn DbIterator<BlockHash, BlockWithSideband>,
            &mut dyn DbIterator<BlockHash, BlockWithSideband>,
        ) + Send
              + Sync),
    ) {
        parallel_traversal(&|start, end, is_last| {
            let mut transaction = self.env.tx_begin_read();
            let mut begin_it = self.begin_at_hash(&transaction, &start.into());
            let mut end_it = if !is_last {
                self.begin_at_hash(&transaction, &end.into())
            } else {
                self.end()
            };
            action(&mut transaction, begin_it.as_mut(), end_it.as_mut());
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
    transaction: &'a LmdbWriteTransaction,
    block_store: &'a LmdbBlockStore,
}

impl<'a> BlockPredecessorMdbSet<'a> {
    fn new(transaction: &'a LmdbWriteTransaction, block_store: &'a LmdbBlockStore) -> Self {
        Self {
            transaction,
            block_store,
        }
    }

    fn fill_value(&mut self, block: &dyn Block) {
        let hash = block.hash();
        let mut value = MdbVal::new();
        self.block_store
            .block_raw_get(self.transaction, block.previous(), &mut value);
        debug_assert!(value.mv_size != 0);
        let mut data = value.as_slice().to_vec();
        let block_type = BlockType::from_u8(data[0]).unwrap();

        let offset = block_successor_offset(data.len(), block_type);
        data[offset..offset + hash.as_bytes().len()].copy_from_slice(hash.as_bytes());

        self.block_store
            .raw_put(self.transaction, &data, block.previous());
    }
}

impl<'a> BlockVisitor for BlockPredecessorMdbSet<'a> {
    fn send_block(&mut self, block: &crate::SendBlock) {
        self.fill_value(block);
    }

    fn receive_block(&mut self, block: &crate::ReceiveBlock) {
        self.fill_value(block);
    }

    fn open_block(&mut self, _block: &crate::OpenBlock) {
        // Open blocks don't have a predecessor
    }

    fn change_block(&mut self, block: &crate::ChangeBlock) {
        self.fill_value(block);
    }

    fn state_block(&mut self, block: &crate::StateBlock) {
        if !block.previous().is_zero() {
            self.fill_value(block);
        }
    }
}

fn block_successor_offset(entry_size: usize, block_type: BlockType) -> usize {
    entry_size - BlockSideband::serialized_size(block_type)
}
