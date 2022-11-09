use crate::core::{
    Account, Amount, Block, BlockDetails, BlockHash, BlockSideband, Epoch, KeyPair, SendBlock,
};

#[derive(Default)]
pub struct SendBlockBuilder {
    previous: Option<BlockHash>,
    destination: Option<Account>,
    balance: Option<Amount>,
    work: Option<u64>,
}

impl SendBlockBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn previous(mut self, hash: BlockHash) -> Self {
        self.previous = Some(hash);
        self
    }

    pub fn destination(mut self, destination: Account) -> Self {
        self.destination = Some(destination);
        self
    }

    pub fn balance(mut self, balance: Amount) -> Self {
        self.balance = Some(balance);
        self
    }

    pub fn work(mut self, work: u64) -> Self {
        self.work = Some(work);
        self
    }

    pub fn build(self) -> anyhow::Result<SendBlock> {
        let key_pair = KeyPair::new();
        let previous = self.previous.unwrap_or(BlockHash::from(1));
        let destination = self.destination.unwrap_or(Account::from(2));
        let balance = self.balance.unwrap_or(Amount::new(3));
        let mut block = SendBlock::new(
            &previous,
            &destination,
            &balance,
            &key_pair.private_key(),
            &key_pair.public_key(),
            self.work.unwrap_or(4),
        )?;
        let details = BlockDetails::new(Epoch::Epoch0, true, false, false);
        block.set_sideband(BlockSideband::new(
            Account::from(4),
            BlockHash::zero(),
            balance,
            5,
            8,
            details,
            Epoch::Epoch0,
        ));
        Ok(block)
    }
}
