use crate::{
    blocks::send_block::SendBlockArgs,
    work::{WorkPool, STUB_WORK_POOL},
    Account, Amount, Block, BlockDetails, BlockHash, BlockSideband, Epoch, PrivateKey, SavedBlock,
};

pub struct TestLegacySendBlockBuilder {
    previous: Option<BlockHash>,
    destination: Option<Account>,
    balance: Option<Amount>,
    previous_balance: Option<Amount>,
    work: Option<u64>,
    priv_key: Option<PrivateKey>,
}

impl TestLegacySendBlockBuilder {
    pub(super) fn new() -> Self {
        Self {
            previous: None,
            destination: None,
            balance: None,
            previous_balance: None,
            work: None,
            priv_key: None,
        }
    }

    pub fn previous(mut self, hash: BlockHash) -> Self {
        self.previous = Some(hash);
        self
    }

    pub fn destination(mut self, destination: Account) -> Self {
        self.destination = Some(destination);
        self
    }

    pub fn balance(mut self, balance: impl Into<Amount>) -> Self {
        self.balance = Some(balance.into());
        self
    }

    pub fn previous_balance(mut self, balance: Amount) -> Self {
        self.previous_balance = Some(balance);
        self
    }

    pub fn amount(mut self, amount: impl Into<Amount>) -> Self {
        let previous_balance = self
            .previous_balance
            .expect("no previous balance specified");
        self.balance = Some(previous_balance - amount.into());
        self
    }

    pub fn sign(mut self, priv_key: PrivateKey) -> Self {
        self.priv_key = Some(priv_key);
        self
    }

    pub fn work(mut self, work: u64) -> Self {
        self.work = Some(work);
        self
    }

    pub fn build(self) -> Block {
        let priv_key = self.priv_key.unwrap_or_default();
        let previous = self.previous.unwrap_or(BlockHash::from(1));
        let destination = self.destination.unwrap_or(Account::from(2));
        let balance = self.balance.unwrap_or(Amount::raw(3));
        let work = self
            .work
            .unwrap_or_else(|| STUB_WORK_POOL.generate_dev2(previous.into()).unwrap());

        SendBlockArgs {
            key: &priv_key,
            previous,
            destination,
            balance,
            work,
        }
        .into()
    }

    pub fn build_saved(self) -> SavedBlock {
        let block = self.build();

        let details = BlockDetails::new(Epoch::Epoch0, true, false, false);
        let sideband = BlockSideband {
            account: Account::from(4),
            successor: BlockHash::zero(),
            balance: block.balance_field().unwrap(),
            height: 5,
            timestamp: 8.into(),
            details,
            source_epoch: Epoch::Epoch0,
        };
        SavedBlock::new(block, sideband)
    }
}
