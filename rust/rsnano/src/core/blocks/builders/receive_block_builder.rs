use crate::{
    core::{Amount, Block, BlockDetails, BlockHash, BlockSideband, Epoch, KeyPair, ReceiveBlock},
    work::DEV_WORK_POOL,
};

pub struct ReceiveBlockBuilder {
    previous: Option<BlockHash>,
    source: Option<BlockHash>,
    key_pair: Option<KeyPair>,
    work: Option<u64>,
    build_sideband: bool,
}

impl ReceiveBlockBuilder {
    pub fn new() -> Self {
        Self {
            previous: None,
            source: None,
            key_pair: None,
            work: None,
            build_sideband: true,
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

    pub fn sign(mut self, key_pair: &KeyPair) -> Self {
        self.key_pair = Some(key_pair.clone());
        self
    }

    pub fn work(mut self, work: u64) -> Self {
        self.work = Some(work);
        self
    }

    pub fn without_sideband(mut self) -> Self {
        self.build_sideband = false;
        self
    }

    pub fn build(self) -> anyhow::Result<ReceiveBlock> {
        let key_pair = self.key_pair.unwrap_or_default();
        let previous = self.previous.unwrap_or(BlockHash::from(1));
        let source = self.source.unwrap_or(BlockHash::from(2));
        let work = self
            .work
            .unwrap_or_else(|| DEV_WORK_POOL.generate_dev2(previous.into()).unwrap());

        let mut block = ReceiveBlock::new(
            previous,
            source,
            &key_pair.private_key(),
            &key_pair.public_key(),
            work,
        )?;

        let details = BlockDetails {
            epoch: Epoch::Epoch0,
            is_send: false,
            is_receive: true,
            is_epoch: false,
        };

        if self.build_sideband {
            block.set_sideband(BlockSideband::new(
                block.account(),
                BlockHash::zero(),
                Amount::new(5),
                1,
                2,
                details,
                Epoch::Epoch0,
            ));
        }

        Ok(block)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        core::{Block, BlockBuilder, BlockHash},
        DEV_NETWORK_PARAMS,
    };

    #[test]
    fn receive_block() {
        let block = BlockBuilder::receive().build().unwrap();
        assert_eq!(block.hashables.previous, BlockHash::from(1));
        assert_eq!(block.hashables.source, BlockHash::from(2));
        assert_eq!(DEV_NETWORK_PARAMS.work.validate_entry_block(&block), false);
        assert!(block.sideband().is_some())
    }
}
