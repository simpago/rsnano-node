use crate::{
    blocks::open_block::OpenBlockArgs,
    utils::UnixTimestamp,
    work::{WorkPool, STUB_WORK_POOL},
    Account, Block, BlockDetails, BlockHash, BlockSideband, Epoch, PrivateKey, PublicKey,
    SavedBlock,
};

pub struct TestLegacyOpenBlockBuilder {
    account: Option<Account>,
    representative: Option<PublicKey>,
    source: Option<BlockHash>,
    prv_key: Option<PrivateKey>,
    work: Option<u64>,
}

impl TestLegacyOpenBlockBuilder {
    pub(super) fn new() -> Self {
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

        OpenBlockArgs {
            key: &prv_key,
            source,
            representative,
            work,
        }
        .into()
    }

    pub fn build_saved(self) -> SavedBlock {
        let block = self.build();

        let details = BlockDetails {
            epoch: Epoch::Epoch0,
            is_send: false,
            is_receive: true,
            is_epoch: false,
        };

        let sideband = BlockSideband {
            height: 1,
            timestamp: UnixTimestamp::new(2),
            successor: BlockHash::zero(),
            account: block.account_field().unwrap(),
            balance: 5.into(),
            details,
            source_epoch: Epoch::Epoch0,
        };

        SavedBlock::new(block, sideband)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{work::WORK_THRESHOLDS_STUB, Amount, BlockBase, Signature, TestBlockBuilder};

    #[test]
    fn create_open_block() {
        let block = TestBlockBuilder::legacy_open().build_saved();
        let Block::LegacyOpen(open) = &*block else {
            panic!("not an open block")
        };
        assert_eq!(open.source(), BlockHash::from(1));
        assert_eq!(open.representative(), PublicKey::from(2));
        assert_ne!(open.account(), Account::zero());
        assert_eq!(WORK_THRESHOLDS_STUB.validate_entry_block(&block), true);
        assert_ne!(*open.signature(), Signature::new());

        assert!(block.successor().is_none());
        assert_eq!(block.balance(), Amount::raw(5));
        assert_eq!(block.height(), 1);
        assert_eq!(block.timestamp(), UnixTimestamp::new(2));
        assert_eq!(block.source_epoch(), Epoch::Epoch0);
    }
}
