use crate::{
    epoch_v1_link, epoch_v2_link, utils::UnixTimestamp, Account, AccountInfo, Amount, Block,
    BlockDetails, BlockHash, BlockSideband, Epoch, PrivateKey, PublicKey, SavedBlock,
    TestBlockBuilder, TestLegacyChangeBlockBuilder, TestLegacyOpenBlockBuilder,
    TestLegacyReceiveBlockBuilder, TestLegacySendBlockBuilder, TestStateBlockBuilder,
    DEV_GENESIS_KEY,
};

/// Builds blocks with sideband data as if they were saved in the ledger
pub struct SavedAccountChain {
    priv_key: PrivateKey,
    account: Account,
    balance: Amount,
    representative: PublicKey,
    blocks: Vec<SavedBlock>,
    epoch: Epoch,
}

impl SavedAccountChain {
    pub fn new() -> Self {
        Self::with_priv_key(PrivateKey::new())
    }

    pub fn genesis() -> Self {
        let mut result = Self::with_priv_key(DEV_GENESIS_KEY.clone());
        result.balance = Amount::MAX;
        result.add_block(
            TestBlockBuilder::legacy_open()
                .source(BlockHash::zero())
                .sign(&result.priv_key)
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
            TestBlockBuilder::legacy_open()
                .source(BlockHash::from(123))
                .sign(&self.priv_key)
                .build(),
            Epoch::Epoch0,
        );
    }

    pub fn with_priv_key(key: PrivateKey) -> Self {
        Self {
            account: key.account(),
            balance: Amount::zero(),
            blocks: Vec::new(),
            representative: PublicKey::zero(),
            priv_key: key,
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

    pub fn blocks(&self) -> &[SavedBlock] {
        &self.blocks
    }

    pub fn block(&self, height: u64) -> &SavedBlock {
        &self.blocks[height as usize - 1]
    }

    pub fn try_get_block(&self, height: u64) -> Option<&SavedBlock> {
        if height == 0 {
            return None;
        }
        self.blocks.get(height as usize - 1)
    }

    pub fn latest_block(&self) -> &SavedBlock {
        self.blocks.last().unwrap()
    }

    pub fn add_legacy_change(&mut self, representative: impl Into<PublicKey>) -> &SavedBlock {
        let block = self
            .new_legacy_change_block()
            .representative(representative.into())
            .build();
        self.add_block(block, Epoch::Epoch0)
    }

    pub fn add_legacy_open_from_account(
        &mut self,
        sender_chain: &SavedAccountChain,
    ) -> &SavedBlock {
        self.add_legacy_open_from_account_block(sender_chain, sender_chain.height())
    }

    pub fn add_legacy_open_from_account_block(
        &mut self,
        sender_chain: &SavedAccountChain,
        height: u64,
    ) -> &SavedBlock {
        let send_block = sender_chain.block(height);
        let amount = sender_chain.amount_of_block(height);
        assert_eq!(self.height(), 0);
        assert!(amount > Amount::zero());
        assert_eq!(send_block.destination_or_link(), self.account);
        self.balance = amount;
        let open_block = TestBlockBuilder::legacy_open()
            .source(send_block.hash())
            .sign(&self.priv_key)
            .build();
        self.add_block(open_block, send_block.epoch())
    }

    pub fn add_legacy_receive_from_account(
        &mut self,
        sender_chain: &SavedAccountChain,
    ) -> &SavedBlock {
        self.add_legacy_receive_from_account_block(sender_chain, sender_chain.height())
    }

    pub fn add_legacy_receive_from_self(&mut self) -> &SavedBlock {
        let send_block = self.block(self.height());
        let amount = self.amount_of_block(self.height());
        assert_eq!(send_block.destination_or_link(), self.account);
        self.add_legacy_receive(send_block.hash(), amount, send_block.epoch())
    }

    pub fn add_legacy_receive_from_account_block(
        &mut self,
        sender: &SavedAccountChain,
        height: u64,
    ) -> &SavedBlock {
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
    ) -> &SavedBlock {
        assert!(amount > Amount::zero());
        let block_builder = TestBlockBuilder::legacy_receive()
            .previous(self.frontier())
            .source(source)
            .sign(&self.priv_key);
        self.balance += amount;
        self.add_block(block_builder.build(), source_epoch)
    }

    pub fn add_legacy_send(&mut self) -> &SavedBlock {
        self.add_legacy_send_to(Account::from(42), Amount::raw(1))
    }

    pub fn add_legacy_send_to(&mut self, destination: Account, amount: Amount) -> &SavedBlock {
        let block = self
            .new_legacy_send_block()
            .amount(amount)
            .destination(destination)
            .build();
        self.add_block(block, Epoch::Epoch0)
    }

    pub fn add_state(&mut self) -> &SavedBlock {
        let state = self.new_state_block().build();
        self.add_block(state, Epoch::Epoch0)
    }

    pub fn add_epoch_v1(&mut self) -> &SavedBlock {
        let epoch_block = self.new_epoch1_block().build();
        self.add_block(epoch_block, Epoch::Epoch0)
    }

    pub fn add_epoch_v2(&mut self) -> &SavedBlock {
        let epoch_block = self.new_epoch2_block().build();
        self.add_block(epoch_block, Epoch::Epoch0)
    }

    pub fn new_epoch1_block(&self) -> TestStateBlockBuilder {
        self.new_state_block()
            .link(epoch_v1_link())
            .key(&DEV_GENESIS_KEY)
    }

    pub fn new_epoch2_block(&self) -> TestStateBlockBuilder {
        self.new_state_block()
            .link(epoch_v2_link())
            .key(&DEV_GENESIS_KEY)
    }

    pub fn new_legacy_open_block(&self) -> TestLegacyOpenBlockBuilder {
        TestBlockBuilder::legacy_open()
            .source(BlockHash::from(123))
            .representative(PublicKey::from(456))
            .sign(&self.priv_key)
    }

    pub fn new_state_block(&self) -> TestStateBlockBuilder {
        TestBlockBuilder::state()
            .account(self.account)
            .balance(self.balance)
            .representative(self.representative)
            .link(0)
            .previous(self.frontier())
            .key(&self.priv_key)
    }

    pub fn new_open_block(&self) -> TestStateBlockBuilder {
        TestBlockBuilder::state()
            .account(self.account)
            .balance(42)
            .representative(1234)
            .link(555)
            .previous(0)
            .key(&self.priv_key)
    }

    pub fn new_legacy_send_block(&self) -> TestLegacySendBlockBuilder {
        TestBlockBuilder::legacy_send()
            .previous(self.frontier())
            .destination(Account::from(42))
            .previous_balance(self.balance)
            .amount(1)
            .sign(self.priv_key.clone())
    }

    pub fn new_send_block(&self) -> TestStateBlockBuilder {
        self.new_state_block()
            .previous_balance(self.balance)
            .amount_sent(Amount::raw(1))
            .link(123)
    }

    pub fn new_receive_block(&self) -> TestStateBlockBuilder {
        self.new_state_block()
            .previous_balance(self.balance)
            .balance(self.balance + Amount::raw(1))
            .link(123)
    }

    pub fn new_epoch1_open_block(&self) -> TestStateBlockBuilder {
        TestBlockBuilder::state()
            .account(self.account)
            .balance(0)
            .representative(0)
            .link(epoch_v1_link())
            .previous(0)
            .key(&DEV_GENESIS_KEY)
    }

    pub fn new_legacy_receive_block(&self) -> TestLegacyReceiveBlockBuilder {
        TestBlockBuilder::legacy_receive()
            .previous(self.frontier())
            .source(BlockHash::from(123))
            .sign(&self.priv_key)
    }

    pub fn new_legacy_change_block(&self) -> TestLegacyChangeBlockBuilder {
        TestBlockBuilder::legacy_change()
            .previous(self.frontier())
            .representative(PublicKey::from(42))
            .sign(&self.priv_key)
    }

    pub fn take_blocks(self) -> Vec<SavedBlock> {
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

    pub fn add_block(&mut self, block: Block, source_epoch: Epoch) -> &SavedBlock {
        if let Some(new_balance) = block.balance_field() {
            self.balance = new_balance;
        }

        if block.link_field().unwrap_or_default() == epoch_v1_link() {
            self.epoch = Epoch::Epoch1;
        } else if block.link_field().unwrap_or_default() == epoch_v2_link() {
            self.epoch = Epoch::Epoch2;
        }

        let sideband = BlockSideband {
            height: self.height() + 1,
            timestamp: UnixTimestamp::new(1),
            successor: BlockHash::zero(),
            account: self.account,
            balance: self.balance,
            details: BlockDetails::new(self.epoch, false, false, false),
            source_epoch,
        };

        if !self.blocks.is_empty() {
            let previous = self.blocks.last_mut().unwrap();
            let mut sideband = previous.sideband.clone();
            sideband.successor = block.hash();
            previous.set_sideband(sideband);
        }

        if let Some(rep) = block.representative_field() {
            self.representative = rep;
        }

        self.blocks.push(SavedBlock::new(block, sideband));
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

impl Default for SavedAccountChain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BlockType;

    #[test]
    fn default_account() {
        let chain1 = SavedAccountChain::new();
        let chain2 = SavedAccountChain::new();
        assert_ne!(chain1.account, chain2.account);
    }

    #[test]
    fn add_legacy_open() {
        let mut genesis = SavedAccountChain::genesis();
        let mut chain = SavedAccountChain::new();
        genesis.add_legacy_send_to(chain.account, Amount::raw(10));
        chain.add_legacy_open_from_account(&genesis);
        let block = chain.latest_block();
        assert_eq!(block.account_field(), Some(chain.account()));
        assert_eq!(block.block_type(), BlockType::LegacyOpen);
        assert_eq!(block.height(), 1);
        assert_eq!(chain.frontier(), block.hash());
        assert_eq!(chain.height(), 1);
        assert_eq!(
            chain.account_info().representative,
            block.representative_field().unwrap()
        );
    }
}
