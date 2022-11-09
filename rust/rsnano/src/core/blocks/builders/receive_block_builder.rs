use crate::core::{
    Amount, Block, BlockDetails, BlockHash, BlockSideband, Epoch, KeyPair, ReceiveBlock,
};

#[derive(Default)]
pub struct ReceiveBlockBuilder {
    previous: Option<BlockHash>,
}

impl ReceiveBlockBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn previous(mut self, previous: BlockHash) -> Self {
        self.previous = Some(previous);
        self
    }

    pub fn build(self) -> anyhow::Result<ReceiveBlock> {
        let key_pair = KeyPair::new();

        let previous = self.previous.unwrap_or(BlockHash::from(1));
        let mut block = ReceiveBlock::new(
            previous,
            BlockHash::from(2),
            &key_pair.private_key(),
            &key_pair.public_key(),
            4,
        )?;

        let details = BlockDetails {
            epoch: Epoch::Epoch0,
            is_send: false,
            is_receive: true,
            is_epoch: false,
        };
        block.set_sideband(BlockSideband::new(
            block.account(),
            BlockHash::zero(),
            Amount::new(5),
            1,
            2,
            details,
            Epoch::Epoch0,
        ));

        Ok(block)
    }
}

#[cfg(test)]
mod tests {
    use crate::core::{Block, BlockBuilder, BlockHash};

    #[test]
    fn receive_block() {
        let block = BlockBuilder::receive().build().unwrap();
        assert_eq!(block.hashables.previous, BlockHash::from(1));
        assert_eq!(block.hashables.source, BlockHash::from(2));
        assert_eq!(block.work, 4);
        assert!(block.sideband().is_some())
    }
}
