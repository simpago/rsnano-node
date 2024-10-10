use crate::{
    work::{WorkPool, STUB_WORK_POOL},
    Account, Amount, Block, BlockDetails, BlockEnum, BlockHash, BlockSideband, Epoch, KeyPair,
    SendBlock,
};

pub struct LegacySendBlockBuilder {
    account: Option<Account>,
    height: Option<u64>,
    previous: Option<BlockHash>,
    destination: Option<Account>,
    balance: Option<Amount>,
    previous_balance: Option<Amount>,
    work: Option<u64>,
    keypair: Option<KeyPair>,
    build_sideband: bool,
}

impl LegacySendBlockBuilder {
    pub fn new() -> Self {
        Self {
            account: None,
            height: None,
            previous: None,
            destination: None,
            balance: None,
            previous_balance: None,
            work: None,
            keypair: None,
            build_sideband: false,
        }
    }

    pub fn account(mut self, account: Account) -> Self {
        self.account = Some(account);
        self
    }

    pub fn height(mut self, height: u64) -> Self {
        self.height = Some(height);
        self
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

    pub fn sign(mut self, keypair: KeyPair) -> Self {
        self.keypair = Some(keypair);
        self
    }

    pub fn work(mut self, work: u64) -> Self {
        self.work = Some(work);
        self
    }

    pub fn with_sideband(mut self) -> Self {
        self.build_sideband = true;
        self
    }

    pub fn build(self) -> BlockEnum {
        let key_pair = self.keypair.unwrap_or_default();
        let previous = self.previous.unwrap_or(BlockHash::from(1));
        let destination = self.destination.unwrap_or(Account::from(2));
        let balance = self.balance.unwrap_or(Amount::raw(3));
        let work = self
            .work
            .unwrap_or_else(|| STUB_WORK_POOL.generate_dev2(previous.into()).unwrap());
        let mut block = SendBlock::new(
            &previous,
            &destination,
            &balance,
            &key_pair.private_key(),
            work,
        );

        if self.build_sideband || self.account.is_some() || self.height.is_some() {
            let details = BlockDetails::new(Epoch::Epoch0, true, false, false);
            block.set_sideband(BlockSideband::new(
                self.account.unwrap_or(Account::from(4)),
                BlockHash::zero(),
                balance,
                self.height.unwrap_or(5),
                8,
                details,
                Epoch::Epoch0,
            ));
        }
        BlockEnum::LegacySend(block)
    }
}
