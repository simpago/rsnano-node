use crate::core::{
    Account, Amount, Block, BlockDetails, BlockHash, BlockSideband, Epoch, KeyPair, OpenBlock,
};

#[derive(Default)]
pub struct OpenBlockBuilder {
    account: Option<Account>,
    representative: Option<Account>,
}

impl OpenBlockBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn account(mut self, account: Account) -> Self {
        self.account = Some(account);
        self
    }

    pub fn representative(mut self, representative: Account) -> Self {
        self.representative = Some(representative);
        self
    }

    pub fn build(self) -> anyhow::Result<OpenBlock> {
        let key_pair = KeyPair::new();
        let account = self.account.unwrap_or_else(|| key_pair.public_key().into());
        let representative = self.representative.unwrap_or(Account::from(2));

        let mut block = OpenBlock::new(
            BlockHash::from(1),
            representative,
            account,
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
    use super::*;
    use crate::core::{BlockBuilder, Signature};

    #[test]
    fn create_open_block() {
        let block = BlockBuilder::open().build().unwrap();
        assert_eq!(block.hashables.source, BlockHash::from(1));
        assert_eq!(block.hashables.representative, Account::from(2));
        assert_ne!(block.hashables.account, Account::zero());
        assert_eq!(block.work, 4);
        assert_ne!(*block.block_signature(), Signature::new());

        let sideband = block.sideband().unwrap();
        assert_eq!(sideband.account, block.account());
        assert!(sideband.successor.is_zero());
        assert_eq!(sideband.balance, Amount::new(5));
        assert_eq!(sideband.height, 1);
        assert_eq!(sideband.timestamp, 2);
        assert_eq!(sideband.source_epoch, Epoch::Epoch0);
    }
}
