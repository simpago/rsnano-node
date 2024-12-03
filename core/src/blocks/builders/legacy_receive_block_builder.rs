use crate::{
    work::{WorkPool, STUB_WORK_POOL},
    Block, BlockHash, PrivateKey, ReceiveBlock,
};

pub struct LegacyReceiveBlockBuilder {
    previous: Option<BlockHash>,
    source: Option<BlockHash>,
    key_pair: Option<PrivateKey>,
    work: Option<u64>,
}

impl LegacyReceiveBlockBuilder {
    pub fn new() -> Self {
        Self {
            previous: None,
            source: None,
            key_pair: None,
            work: None,
        }
    }

    pub fn previous(mut self, previous: BlockHash) -> Self {
        self.previous = Some(previous);
        self
    }

    pub fn source(mut self, source: BlockHash) -> Self {
        self.source = Some(source);
        self
    }

    pub fn sign(mut self, key_pair: &PrivateKey) -> Self {
        self.key_pair = Some(key_pair.clone());
        self
    }

    pub fn work(mut self, work: u64) -> Self {
        self.work = Some(work);
        self
    }

    pub fn build(self) -> Block {
        let key_pair = self.key_pair.unwrap_or_default();
        let previous = self.previous.unwrap_or(BlockHash::from(1));
        let source = self.source.unwrap_or(BlockHash::from(2));
        let work = self
            .work
            .unwrap_or_else(|| STUB_WORK_POOL.generate_dev2(previous.into()).unwrap());

        let block = ReceiveBlock::new(previous, source, &key_pair, work);
        Block::LegacyReceive(block)
    }
}

impl Default for LegacyReceiveBlockBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::{work::WORK_THRESHOLDS_STUB, Block, BlockBuilder, BlockHash};

    #[test]
    fn receive_block() {
        let block = BlockBuilder::legacy_receive().build();
        let Block::LegacyReceive(receive) = &block else {
            panic!("not a receive block!")
        };
        assert_eq!(receive.hashables.previous, BlockHash::from(1));
        assert_eq!(receive.hashables.source, BlockHash::from(2));
        assert_eq!(WORK_THRESHOLDS_STUB.validate_entry_block(&block), true);
    }
}
