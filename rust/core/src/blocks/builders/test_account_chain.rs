use crate::{
    epoch_v1_link, epoch_v2_link, Account, AccountInfo, Amount, BlockBuilder, BlockDetails,
    BlockEnum, BlockHash, BlockSideband, Epoch, KeyPair, LegacyChangeBlockBuilder,
    LegacyOpenBlockBuilder, LegacyReceiveBlockBuilder, LegacySendBlockBuilder, PublicKey,
    StateBlockBuilder, DEV_GENESIS_KEY,
};

pub struct TestAccountChain {
    keypair: KeyPair,
    account: Account,
    balance: Amount,
    representative: PublicKey,
    blocks: Vec<BlockEnum>,
    epoch: Epoch,
}

impl TestAccountChain {
    pub fn new() -> Self {
        Self::with_keys(KeyPair::new())
    }

    pub fn genesis() -> Self {
        let mut result = Self::with_keys(DEV_GENESIS_KEY.clone());
        result.balance = Amount::MAX;
        result.add_block(
            BlockBuilder::legacy_open()
                .account(result.account)
                .source(BlockHash::zero())
                .sign(&result.keypair)
                .build(),
            Epoch::Epoch0,
        );
        result
    }

    pub fn new_opened_chain() -> Self {
        let mut result = Self::new();
        result.add_random_open_block();
        result
    }

    pub fn add_random_open_block(&mut self) {
        assert_eq!(self.height(), 0);
        self.balance = Amount::nano(1);
        self.add_block(
            BlockBuilder::legacy_open()
                .account(self.account)
                .source(BlockHash::from(123))
                .sign(&self.keypair)
                .build(),
            Epoch::Epoch0,
        );
    }

    pub fn with_keys(keypair: KeyPair) -> Self {
        Self {
            account: keypair.account(),
            balance: Amount::zero(),
            blocks: Vec::new(),
            representative: PublicKey::zero(),
            keypair,
            epoch: Epoch::Epoch0,
        }
    }

    pub fn height(&self) -> u64 {
        self.blocks.len() as u64
    }

    pub fn open(&self) -> BlockHash {
        self.blocks[0].hash()
    }

    pub fn frontier(&self) -> BlockHash {
        self.blocks.last().map(|b| b.hash()).unwrap_or_default()
    }

    pub fn account(&self) -> Account {
        self.account
    }

    pub fn blocks(&self) -> &[BlockEnum] {
        &self.blocks
    }

    pub fn block(&self, height: u64) -> &BlockEnum {
        &self.blocks[height as usize - 1]
    }

    pub fn try_get_block(&self, height: u64) -> Option<&BlockEnum> {
        if height == 0 {
            return None;
        }
        self.blocks.get(height as usize - 1)
    }

    pub fn latest_block(&self) -> &BlockEnum {
        self.blocks.last().unwrap()
    }

    pub fn add_legacy_change(&mut self, representative: impl Into<PublicKey>) -> &BlockEnum {
        let block = self
            .new_legacy_change_block()
            .representative(representative.into())
            .build();
        self.add_block(block, Epoch::Epoch0)
    }

    pub fn add_legacy_open_from_account(&mut self, sender_chain: &TestAccountChain) -> &BlockEnum {
        self.add_legacy_open_from_account_block(sender_chain, sender_chain.height())
    }

    pub fn add_legacy_open_from_account_block(
        &mut self,
        sender_chain: &TestAccountChain,
        height: u64,
    ) -> &BlockEnum {
        let send_block = sender_chain.block(height);
        let amount = sender_chain.amount_of_block(height);
        assert_eq!(self.height(), 0);
        assert!(amount > Amount::zero());
        assert_eq!(send_block.destination_or_link(), self.account);
        self.balance = amount;
        let open_block = BlockBuilder::legacy_open()
            .account(self.account)
            .source(send_block.hash())
            .sign(&self.keypair)
            .build();
        self.add_block(open_block, send_block.epoch())
    }

    pub fn add_legacy_receive_from_account(
        &mut self,
        sender_chain: &TestAccountChain,
    ) -> &BlockEnum {
        self.add_legacy_receive_from_account_block(sender_chain, sender_chain.height())
    }

    pub fn add_legacy_receive_from_self(&mut self) -> &BlockEnum {
        let send_block = self.block(self.height());
        let amount = self.amount_of_block(self.height());
        assert_eq!(send_block.destination_or_link(), self.account);
        self.add_legacy_receive(send_block.hash(), amount, send_block.epoch())
    }

    pub fn add_legacy_receive_from_account_block(
        &mut self,
        sender: &TestAccountChain,
        height: u64,
    ) -> &BlockEnum {
        let send_block = sender.block(height);
        let amount = sender.amount_of_block(height);
        assert_eq!(send_block.destination_or_link(), self.account);
        self.add_legacy_receive(send_block.hash(), amount, send_block.epoch())
    }

    fn add_legacy_receive(
        &mut self,
        source: BlockHash,
        amount: Amount,
        source_epoch: Epoch,
    ) -> &BlockEnum {
        assert!(amount > Amount::zero());
        let block_builder = BlockBuilder::legacy_receive()
            .previous(self.frontier())
            .source(source)
            .sign(&self.keypair);
        self.balance += amount;
        self.add_block(block_builder.build(), source_epoch)
    }

    pub fn add_legacy_send(&mut self) -> &BlockEnum {
        self.add_legacy_send_to(Account::from(42), Amount::raw(1))
    }

    pub fn add_legacy_send_to(&mut self, destination: Account, amount: Amount) -> &BlockEnum {
        let block = self
            .new_legacy_send_block()
            .amount(amount)
            .destination(destination)
            .build();
        self.add_block(block, Epoch::Epoch0)
    }

    pub fn add_state(&mut self) -> &BlockEnum {
        let state = self.new_state_block().build();
        self.add_block(state, Epoch::Epoch0)
    }

    pub fn add_epoch_v1(&mut self) -> &BlockEnum {
        let epoch_block = self.new_epoch1_block().build();
        self.add_block(epoch_block, Epoch::Epoch0)
    }

    pub fn add_epoch_v2(&mut self) -> &BlockEnum {
        let epoch_block = self.new_epoch2_block().build();
        self.add_block(epoch_block, Epoch::Epoch0)
    }

    pub fn new_epoch1_block(&self) -> StateBlockBuilder {
        self.new_state_block()
            .link(epoch_v1_link())
            .sign(&DEV_GENESIS_KEY)
    }

    pub fn new_epoch2_block(&self) -> StateBlockBuilder {
        self.new_state_block()
            .link(epoch_v2_link())
            .sign(&DEV_GENESIS_KEY)
    }

    pub fn new_legacy_open_block(&self) -> LegacyOpenBlockBuilder {
        BlockBuilder::legacy_open()
            .account(self.account)
            .source(BlockHash::from(123))
            .representative(PublicKey::from(456))
            .sign(&self.keypair)
    }

    pub fn new_state_block(&self) -> StateBlockBuilder {
        BlockBuilder::state()
            .account(self.account)
            .balance(self.balance)
            .representative(self.representative)
            .link(0)
            .previous(self.frontier())
            .sign(&self.keypair)
    }

    pub fn new_open_block(&self) -> StateBlockBuilder {
        BlockBuilder::state()
            .account(self.account)
            .balance(42)
            .representative(1234)
            .link(555)
            .previous(0)
            .sign(&self.keypair)
    }

    pub fn new_legacy_send_block(&self) -> LegacySendBlockBuilder {
        BlockBuilder::legacy_send()
            .account(self.account)
            .previous(self.frontier())
            .destination(Account::from(42))
            .previous_balance(self.balance)
            .amount(1)
            .sign(self.keypair.clone())
    }

    pub fn new_send_block(&self) -> StateBlockBuilder {
        self.new_state_block()
            .previous_balance(self.balance)
            .amount_sent(Amount::raw(1))
            .link(123)
    }

    pub fn new_receive_block(&self) -> StateBlockBuilder {
        self.new_state_block()
            .previous_balance(self.balance)
            .balance(self.balance + Amount::raw(1))
            .link(123)
    }

    pub fn new_epoch1_open_block(&self) -> StateBlockBuilder {
        BlockBuilder::state()
            .account(self.account)
            .balance(0)
            .representative(0)
            .link(epoch_v1_link())
            .previous(0)
            .sign(&DEV_GENESIS_KEY)
    }

    pub fn new_legacy_receive_block(&self) -> LegacyReceiveBlockBuilder {
        BlockBuilder::legacy_receive()
            .previous(self.frontier())
            .source(BlockHash::from(123))
            .sign(&self.keypair)
    }

    pub fn new_legacy_change_block(&self) -> LegacyChangeBlockBuilder {
        BlockBuilder::legacy_change()
            .previous(self.frontier())
            .representative(PublicKey::from(42))
            .sign(&self.keypair)
    }

    pub fn take_blocks(self) -> Vec<BlockEnum> {
        self.blocks
    }

    pub fn account_info(&self) -> AccountInfo {
        AccountInfo {
            head: self.frontier(),
            representative: self.representative,
            open_block: self.open(),
            balance: self.latest_block().balance(),
            modified: 123,
            block_count: self.height(),
            epoch: self.epoch,
        }
    }

    fn amount_of_block(&self, height: u64) -> Amount {
        let balance = self.balance_on_height(height);
        let previous_balance = self.balance_on_height(height - 1);
        if balance > previous_balance {
            balance - previous_balance
        } else {
            previous_balance - balance
        }
    }

    fn balance_on_height(&self, height: u64) -> Amount {
        if height == 0 {
            Amount::zero()
        } else {
            self.blocks[height as usize - 1].balance()
        }
    }

    pub fn add_block(&mut self, mut block: BlockEnum, source_epoch: Epoch) -> &BlockEnum {
        if let Some(new_balance) = block.balance_field() {
            self.balance = new_balance;
        }

        if block.link_field().unwrap_or_default() == epoch_v1_link() {
            self.epoch = Epoch::Epoch1;
        } else if block.link_field().unwrap_or_default() == epoch_v2_link() {
            self.epoch = Epoch::Epoch2;
        }

        block.set_sideband(BlockSideband {
            height: self.height() + 1,
            timestamp: 1,
            successor: BlockHash::zero(),
            account: self.account,
            balance: self.balance,
            details: BlockDetails::new(self.epoch, false, false, false),
            source_epoch,
        });

        if !self.blocks.is_empty() {
            let previous = self.blocks.last_mut().unwrap();
            let mut sideband = previous.sideband().unwrap().clone();
            sideband.successor = block.hash();
            previous.set_sideband(sideband);
        }

        if let Some(rep) = block.representative_field() {
            self.representative = rep;
        }

        self.blocks.push(block);
        self.blocks.last().unwrap()
    }

    pub fn representative_at_height(&self, height: u64) -> Option<PublicKey> {
        self.blocks[..height as usize]
            .iter()
            .rev()
            .filter_map(|b| b.representative_field())
            .next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BlockType;

    #[test]
    fn default_account() {
        let chain1 = TestAccountChain::new();
        let chain2 = TestAccountChain::new();
        assert_ne!(chain1.account, chain2.account);
    }

    #[test]
    fn add_legacy_open() {
        let mut genesis = TestAccountChain::genesis();
        let mut chain = TestAccountChain::new();
        genesis.add_legacy_send_to(chain.account, Amount::raw(10));
        chain.add_legacy_open_from_account(&genesis);
        let block = chain.latest_block();
        assert_eq!(block.account_field(), Some(chain.account()));
        assert_eq!(block.block_type(), BlockType::LegacyOpen);
        assert_eq!(block.sideband().unwrap().height, 1);
        assert_eq!(chain.frontier(), block.hash());
        assert_eq!(chain.height(), 1);
        assert_eq!(
            chain.account_info().representative,
            block.representative_field().unwrap()
        );
    }
}
