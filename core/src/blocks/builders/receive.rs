use crate::{
    blocks::receive_block::ReceiveBlockArgs,
    work::{WorkPool, STUB_WORK_POOL},
    Block, BlockHash, PrivateKey,
};

pub struct TestLegacyReceiveBlockBuilder {
    previous: Option<BlockHash>,
    source: Option<BlockHash>,
    key_pair: Option<PrivateKey>,
    work: Option<u64>,
}

impl TestLegacyReceiveBlockBuilder {
    pub(super) fn new() -> Self {
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
        let key = self.key_pair.unwrap_or_default();
        let previous = self.previous.unwrap_or(BlockHash::from(1));
        let source = self.source.unwrap_or(BlockHash::from(2));
        let work = self
            .work
            .unwrap_or_else(|| STUB_WORK_POOL.generate_dev2(previous.into()).unwrap());

        ReceiveBlockArgs {
            key: &key,
            previous,
            source,
            work,
        }
        .into()
    }
}

#[cfg(test)]
mod tests {
    use crate::{work::WORK_THRESHOLDS_STUB, Block, BlockBase, BlockHash, TestBlockBuilder};

    #[test]
    fn receive_block() {
        let block = TestBlockBuilder::legacy_receive().build();
        let Block::LegacyReceive(receive) = &block else {
            panic!("not a receive block!")
        };
        assert_eq!(receive.previous(), BlockHash::from(1));
        assert_eq!(receive.source(), BlockHash::from(2));
        assert_eq!(WORK_THRESHOLDS_STUB.validate_entry_block(&block), true);
    }
}
