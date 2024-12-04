use crate::{
    work::{WorkPool, STUB_WORK_POOL},
    Account, Amount, Block, BlockDetails, BlockHash, BlockSideband, Epoch, OpenBlock, PrivateKey,
    PublicKey, SavedBlock,
};

pub struct LegacyOpenBlockBuilder {
    account: Option<Account>,
    representative: Option<PublicKey>,
    source: Option<BlockHash>,
    prv_key: Option<PrivateKey>,
    work: Option<u64>,
}

impl LegacyOpenBlockBuilder {
    pub fn new() -> Self {
        Self {
            account: None,
            representative: None,
            source: None,
            prv_key: None,
            work: None,
        }
    }

    pub fn source(mut self, source: BlockHash) -> Self {
        self.source = Some(source);
        self
    }

    pub fn account(mut self, account: Account) -> Self {
        self.account = Some(account);
        self
    }

    pub fn representative(mut self, representative: PublicKey) -> Self {
        self.representative = Some(representative);
        self
    }

    pub fn sign(mut self, prv_key: &PrivateKey) -> Self {
        self.prv_key = Some(prv_key.clone());
        self
    }

    pub fn work(mut self, work: u64) -> Self {
        self.work = Some(work);
        self
    }
    pub fn build(self) -> Block {
        let source = self.source.unwrap_or(BlockHash::from(1));
        let prv_key = self.prv_key.unwrap_or_default();
        let account = self.account.unwrap_or_else(|| prv_key.account());
        let representative = self.representative.unwrap_or(PublicKey::from(2));
        let work = self
            .work
            .unwrap_or_else(|| STUB_WORK_POOL.generate_dev2(account.into()).unwrap());

        let block = OpenBlock::new(source, representative, account, &prv_key, work);
        Block::LegacyOpen(block)
    }

    pub fn build_saved(self) -> SavedBlock {
        let block = self.build();

        let details = BlockDetails {
            epoch: Epoch::Epoch0,
            is_send: false,
            is_receive: true,
            is_epoch: false,
        };

        let sideband = BlockSideband::new(
            block.account_field().unwrap(),
            BlockHash::zero(),
            Amount::raw(5),
            1,
            2,
            details,
            Epoch::Epoch0,
        );

        SavedBlock::new(block, sideband)
    }
}

impl Default for LegacyOpenBlockBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{work::WORK_THRESHOLDS_STUB, BlockBase, BlockBuilder, Signature};

    #[test]
    fn create_open_block() {
        let block = BlockBuilder::legacy_open().build_saved();
        let Block::LegacyOpen(open) = &*block else {
            panic!("not an open block")
        };
        assert_eq!(open.hashables.source, BlockHash::from(1));
        assert_eq!(open.hashables.representative, PublicKey::from(2));
        assert_ne!(open.account(), Account::zero());
        assert_eq!(WORK_THRESHOLDS_STUB.validate_entry_block(&block), true);
        assert_ne!(*open.block_signature(), Signature::new());

        assert!(block.successor().is_none());
        assert_eq!(block.balance(), Amount::raw(5));
        assert_eq!(block.height(), 1);
        assert_eq!(block.timestamp(), 2);
        assert_eq!(block.source_epoch(), Epoch::Epoch0);
    }
}
