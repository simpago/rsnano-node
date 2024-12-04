use crate::work::WorkPool;
use crate::{work::STUB_WORK_POOL, BlockHash, ChangeBlock};
use crate::{Account, Block, PrivateKey, PublicKey};

pub struct LegacyChangeBlockBuilder {
    account: Option<Account>,
    representative: Option<PublicKey>,
    previous: Option<BlockHash>,
    prv_key: Option<PrivateKey>,
    work: Option<u64>,
}

impl LegacyChangeBlockBuilder {
    pub fn new() -> Self {
        Self {
            account: None,
            representative: None,
            previous: None,
            prv_key: None,
            work: None,
        }
    }

    pub fn previous(mut self, previous: BlockHash) -> Self {
        self.previous = Some(previous);
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

    pub fn sign(mut self, keypair: &PrivateKey) -> Self {
        self.prv_key = Some(keypair.clone());
        self
    }

    pub fn work(mut self, work: u64) -> Self {
        self.work = Some(work);
        self
    }

    pub fn build(self) -> Block {
        let previous = self.previous.unwrap_or(BlockHash::from(1));
        let prv_key = self.prv_key.unwrap_or_default();
        let representative = self.representative.unwrap_or(PublicKey::from(2));
        let work = self
            .work
            .unwrap_or_else(|| STUB_WORK_POOL.generate_dev2(previous.into()).unwrap());

        let block = ChangeBlock::new(previous, representative, &prv_key, work);
        Block::LegacyChange(block)
    }
}

impl Default for LegacyChangeBlockBuilder {
    fn default() -> Self {
        Self::new()
    }
}
