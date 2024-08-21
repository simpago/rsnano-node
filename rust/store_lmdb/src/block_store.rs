use crate::{
    BinaryDbIterator, LmdbDatabase, LmdbEnv, LmdbIteratorImpl, LmdbWriteTransaction, Transaction,
    BLOCK_TEST_DATABASE,
};
use lmdb::{DatabaseFlags, WriteFlags};
use num_traits::FromPrimitive;
use rsnano_core::{
    utils::{BufferReader, FixedSizeSerialize},
    Block, BlockEnum, BlockHash, BlockSideband, BlockType, BlockVisitor, BlockWithSideband,
    ChangeBlock, OpenBlock, ReceiveBlock, SendBlock, StateBlock,
};
use rsnano_nullable_lmdb::ConfiguredDatabase;
#[cfg(feature = "output_tracking")]
use rsnano_output_tracker::{OutputListenerMt, OutputTrackerMt};
use std::sync::Arc;

pub type BlockIterator<'txn> = BinaryDbIterator<'txn, BlockHash, BlockWithSideband>;

pub struct LmdbBlockStore {
    _env: Arc<LmdbEnv>,
    database: LmdbDatabase,
    #[cfg(feature = "output_tracking")]
    put_listener: OutputListenerMt<BlockEnum>,
}

pub struct ConfiguredBlockDatabaseBuilder {
    database: ConfiguredDatabase,
}

impl ConfiguredBlockDatabaseBuilder {
    pub fn new() -> Self {
        Self {
            database: ConfiguredDatabase::new(BLOCK_TEST_DATABASE, "blocks"),
        }
    }

    pub fn block(mut self, block: &BlockEnum) -> Self {
        self.database.entries.insert(
            block.hash().as_bytes().to_vec(),
            block.serialize_with_sideband(),
        );
        self
    }

    pub fn build(self) -> ConfiguredDatabase {
        self.database
    }
}

impl LmdbBlockStore {
    pub fn configured_responses() -> ConfiguredBlockDatabaseBuilder {
        ConfiguredBlockDatabaseBuilder::new()
    }

    pub fn new(env: Arc<LmdbEnv>) -> anyhow::Result<Self> {
        let database = env
            .environment
            .create_db(Some("blocks"), DatabaseFlags::empty())?;
        Ok(Self {
            _env: env,
            database,
            #[cfg(feature = "output_tracking")]
            put_listener: OutputListenerMt::new(),
        })
    }

    pub fn database(&self) -> LmdbDatabase {
        self.database
    }

    #[cfg(feature = "output_tracking")]
    pub fn track_puts(&self) -> Arc<OutputTrackerMt<BlockEnum>> {
        self.put_listener.track()
    }

    pub fn put(&self, txn: &mut LmdbWriteTransaction, block: &BlockEnum) {
        #[cfg(feature = "output_tracking")]
        self.put_listener.emit(block.clone());

        let hash = block.hash();
        debug_assert!(
            block.sideband().unwrap().successor.is_zero()
                || self.exists(txn, &block.sideband().unwrap().successor)
        );

        self.raw_put(txn, &block.serialize_with_sideband(), &hash);
        {
            let mut predecessor = BlockPredecessorMdbSet::new(txn, self);
            block.visit(&mut predecessor);
        }
    }

    pub fn exists(&self, transaction: &dyn Transaction, hash: &BlockHash) -> bool {
        transaction.exists(self.database, hash.as_bytes())
    }

    pub fn successor(&self, txn: &dyn Transaction, hash: &BlockHash) -> Option<BlockHash> {
        self.block_raw_get(txn, hash).and_then(|data| {
            debug_assert!(data.len() >= 32);
            let block_type = BlockType::from_u8(data[0]).unwrap();
            let offset = block_successor_offset(data.len(), block_type);
            let successor = BlockHash::from_bytes(data[offset..offset + 32].try_into().unwrap());
            if successor.is_zero() {
                None
            } else {
                Some(successor)
            }
        })
    }

    pub fn successor_clear(&self, txn: &mut LmdbWriteTransaction, hash: &BlockHash) {
        let value = self.block_raw_get(txn, hash).unwrap();
        let block_type = BlockType::from_u8(value[0]).unwrap();

        let mut data = value.to_vec();
        let offset = block_successor_offset(value.len(), block_type);
        data[offset..offset + BlockHash::serialized_size()].fill(0);
        self.raw_put(txn, &data, hash)
    }

    pub fn get(&self, txn: &dyn Transaction, hash: &BlockHash) -> Option<BlockEnum> {
        self.block_raw_get(txn, hash).map(|bytes| {
            BlockEnum::deserialize_with_sideband(bytes)
                .unwrap_or_else(|_| panic!("Could not deserialize block {}!", hash))
        })
    }

    pub fn get_no_sideband(&self, txn: &dyn Transaction, hash: &BlockHash) -> Option<BlockEnum> {
        match self.block_raw_get(txn, hash) {
            None => None,
            Some(bytes) => {
                let mut stream = BufferReader::new(bytes);
                Some(BlockEnum::deserialize(&mut stream).unwrap())
            }
        }
    }

    pub fn del(&self, txn: &mut LmdbWriteTransaction, hash: &BlockHash) {
        txn.delete(self.database, hash.as_bytes(), None).unwrap();
    }

    pub fn count(&self, txn: &dyn Transaction) -> u64 {
        txn.count(self.database)
    }

    pub fn begin<'txn>(&self, transaction: &'txn dyn Transaction) -> BlockIterator<'txn> {
        LmdbIteratorImpl::new_iterator(transaction, self.database, None, true)
    }

    pub fn begin_at_hash<'txn>(
        &self,
        transaction: &'txn dyn Transaction,
        hash: &BlockHash,
    ) -> BlockIterator<'txn> {
        LmdbIteratorImpl::new_iterator(transaction, self.database, Some(hash.as_bytes()), true)
    }

    pub fn end(&self) -> BlockIterator {
        LmdbIteratorImpl::null_iterator()
    }

    pub fn random(&self, transaction: &dyn Transaction) -> Option<BlockEnum> {
        let hash = BlockHash::random();
        let mut existing = self.begin_at_hash(transaction, &hash);
        if existing.is_end() {
            existing = self.begin(transaction);
        }

        existing.current().map(|(_, v)| v.block.clone())
    }

    pub fn raw_put(&self, txn: &mut LmdbWriteTransaction, data: &[u8], hash: &BlockHash) {
        txn.put(self.database, hash.as_bytes(), data, WriteFlags::empty())
            .unwrap();
    }

    pub fn block_raw_get<'a>(
        &self,
        txn: &'a dyn Transaction,
        hash: &BlockHash,
    ) -> Option<&'a [u8]> {
        match txn.get(self.database, hash.as_bytes()) {
            Err(lmdb::Error::NotFound) => None,
            Ok(bytes) => Some(bytes),
            Err(e) => panic!("Could not load block. {:?}", e),
        }
    }
}

/// Fill in our predecessors
struct BlockPredecessorMdbSet<'a> {
    transaction: &'a mut LmdbWriteTransaction,
    block_store: &'a LmdbBlockStore,
}

impl<'a> BlockPredecessorMdbSet<'a> {
    fn new(transaction: &'a mut LmdbWriteTransaction, block_store: &'a LmdbBlockStore) -> Self {
        Self {
            transaction,
            block_store,
        }
    }

    fn fill_value(&mut self, block: &dyn Block) {
        let hash = block.hash();
        let value = self
            .block_store
            .block_raw_get(self.transaction, &block.previous())
            .expect("block not found by fill_value");
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
    use crate::PutEvent;
    use rsnano_core::BlockBuilder;

    use super::*;

    struct Fixture {
        env: Arc<LmdbEnv>,
        store: LmdbBlockStore,
    }

    impl Fixture {
        fn new() -> Self {
            Self::with_env(LmdbEnv::new_null())
        }

        fn with_env(env: LmdbEnv) -> Self {
            let env = Arc::new(env);
            Self {
                env: env.clone(),
                store: LmdbBlockStore::new(env).unwrap(),
            }
        }
    }

    #[test]
    fn empty() {
        let fixture = Fixture::new();
        let store = &fixture.store;
        let txn = fixture.env.tx_begin_read();

        assert!(store.get(&txn, &BlockHash::from(1)).is_none());
        assert_eq!(store.exists(&txn, &BlockHash::from(1)), false);
        assert_eq!(store.count(&txn), 0);
    }

    #[test]
    fn load_block_by_hash() {
        let block = BlockBuilder::legacy_open().with_sideband().build();

        let env = LmdbEnv::new_null_with()
            .database("blocks", LmdbDatabase::new_null(100))
            .entry(block.hash().as_bytes(), &block.serialize_with_sideband())
            .build()
            .build();
        let fixture = Fixture::with_env(env);
        let txn = fixture.env.tx_begin_read();

        let result = fixture.store.get(&txn, &block.hash());
        assert_eq!(result, Some(block));
    }

    #[test]
    fn add_block() {
        let fixture = Fixture::new();
        let mut txn = fixture.env.tx_begin_write();
        let put_tracker = txn.track_puts();
        let block = BlockBuilder::legacy_open().with_sideband().build();

        fixture.store.put(&mut txn, &block);

        assert_eq!(
            put_tracker.output(),
            vec![PutEvent {
                database: LmdbDatabase::new_null(42),
                key: block.hash().as_bytes().to_vec(),
                value: block.serialize_with_sideband(),
                flags: lmdb::WriteFlags::empty(),
            }]
        );
    }

    #[test]
    fn clear_successor() {
        let mut block = BlockBuilder::legacy_open().build();
        let sideband = BlockSideband {
            successor: BlockHash::from(123),
            ..BlockSideband::new_test_instance()
        };
        block.set_sideband(sideband.clone());

        let env = LmdbEnv::new_null_with()
            .database("blocks", LmdbDatabase::new_null(100))
            .entry(block.hash().as_bytes(), &block.serialize_with_sideband())
            .build()
            .build();
        let fixture = Fixture::with_env(env);
        let mut txn = fixture.env.tx_begin_write();
        let put_tracker = txn.track_puts();

        fixture.store.successor_clear(&mut txn, &block.hash());

        let mut expected_block = block.clone();
        expected_block.set_sideband(BlockSideband {
            successor: BlockHash::zero(),
            ..sideband
        });

        assert_eq!(
            put_tracker.output(),
            vec![PutEvent {
                database: LmdbDatabase::new_null(100),
                key: expected_block.hash().as_bytes().to_vec(),
                value: expected_block.serialize_with_sideband(),
                flags: WriteFlags::empty(),
            }]
        );
    }

    #[test]
    fn random() -> anyhow::Result<()> {
        let block = BlockBuilder::legacy_open().with_sideband().build();

        let env = LmdbEnv::new_null_with()
            .database("blocks", LmdbDatabase::new_null(100))
            .entry(block.hash().as_bytes(), &block.serialize_with_sideband())
            .build()
            .build();

        let fixture = Fixture::with_env(env);
        let txn = fixture.env.tx_begin_read();

        let random = fixture.store.random(&txn).expect("block not found");

        assert_eq!(random, block);
        Ok(())
    }

    #[test]
    fn track_inserted_blocks() {
        let fixture = Fixture::new();
        let mut block = BlockBuilder::state().previous(BlockHash::zero()).build();
        block.set_sideband(BlockSideband {
            height: 1,
            successor: BlockHash::zero(),
            ..BlockSideband::new_test_instance()
        });
        let mut txn = fixture.env.tx_begin_write();
        let put_tracker = fixture.store.track_puts();

        fixture.store.put(&mut txn, &block);

        assert_eq!(put_tracker.output(), vec![block]);
    }

    #[test]
    fn can_be_nulled() {
        let block = BlockBuilder::state().with_sideband().build();
        let configured_responses = LmdbBlockStore::configured_responses().block(&block).build();
        let env = LmdbEnv::new_null_with()
            .configured_database(configured_responses)
            .build();
        let txn = env.tx_begin_read();
        let block_store = LmdbBlockStore::new(Arc::new(env)).unwrap();
        assert_eq!(block_store.get(&txn, &block.hash()), Some(block));
    }
}
