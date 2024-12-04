mod validate_epoch_v1;
mod validate_epoch_v2;
mod validate_legacy_change;
mod validate_legacy_open;
mod validate_legacy_receive;
mod validate_legacy_send;
mod validate_state_change;
mod validate_state_open;
mod validate_state_receive;
mod validate_state_send;

use crate::{
    block_insertion::BlockInsertInstructions, ledger_constants::LEDGER_CONSTANTS_STUB, BlockStatus,
};
use rsnano_core::{
    work::WORK_THRESHOLDS_STUB, Account, Amount, Block, Epoch, PendingInfo, SavedAccountChain,
};

use super::BlockValidator;

pub(crate) struct BlockValidationTest {
    pub seconds_since_epoch: u64,
    pub chain: SavedAccountChain,
    block: Option<Block>,
    pending_receive: Option<PendingInfo>,
    block_already_exists: bool,
    source_block_missing: bool,
    previous_block_missing: bool,
}
impl BlockValidationTest {
    pub fn for_epoch0_account() -> Self {
        let mut result = Self::for_unopened_account();
        result.chain.add_random_open_block();
        result
    }

    pub fn for_epoch1_account() -> Self {
        let mut result = Self::for_unopened_account();
        result.chain.add_random_open_block();
        result.setup_account(|chain| {
            chain.add_epoch_v1();
        })
    }

    pub fn for_epoch2_account() -> Self {
        let mut result = Self::for_unopened_account();
        result.chain.add_random_open_block();
        result.setup_account(|chain| {
            chain.add_epoch_v1();
            chain.add_epoch_v2();
        })
    }

    pub fn for_unopened_account() -> Self {
        Self {
            chain: SavedAccountChain::new(),
            block: None,
            pending_receive: None,
            seconds_since_epoch: 123456,
            block_already_exists: false,
            source_block_missing: false,
            previous_block_missing: false,
        }
    }

    pub fn setup_account(mut self, mut setup: impl FnMut(&mut SavedAccountChain)) -> Self {
        setup(&mut self.chain);
        self
    }

    pub fn block_to_validate(
        mut self,
        create_block: impl FnOnce(&SavedAccountChain) -> Block,
    ) -> Self {
        self.block = Some(create_block(&self.chain));
        self
    }

    pub fn previous_block_is_missing(mut self) -> Self {
        self.previous_block_missing = true;
        self
    }

    pub fn source_block_is_missing(mut self) -> Self {
        self.source_block_missing = true;
        self
    }

    pub fn block_already_exists(mut self) -> Self {
        self.block_already_exists = true;
        self
    }

    pub fn with_pending_receive(mut self, amount: Amount, source_epoch: Epoch) -> Self {
        self.pending_receive = Some(PendingInfo {
            source: Account::from(42),
            amount,
            epoch: source_epoch,
        });
        self
    }

    pub fn block(&self) -> &Block {
        self.block.as_ref().unwrap()
    }

    pub fn assert_validation_fails_with(&self, expected: BlockStatus) {
        assert_eq!(self.validate(), Err(expected));
    }

    pub fn assert_is_valid(&self) -> BlockInsertInstructions {
        self.validate().expect("block should be valid!")
    }

    fn validate(&self) -> Result<BlockInsertInstructions, BlockStatus> {
        let block = self.block.as_ref().unwrap();
        let mut validator = new_test_validator(block, self.chain.account());
        if self.chain.height() > 0 {
            validator.old_account_info = Some(self.chain.account_info());
            if !self.previous_block_missing {
                validator.previous_block = Some(self.chain.latest_block().clone());
            }
        };
        validator.seconds_since_epoch = self.seconds_since_epoch;
        if self.pending_receive.is_some() {
            validator.any_pending_exists = true;
            validator.source_block_exists = true;
            validator.pending_receive_info = self.pending_receive.clone();
        }
        validator.block_exists = self.block_already_exists;
        validator.source_block_exists = !self.source_block_missing;
        validator.validate()
    }
}

fn new_test_validator<'a>(block: &'a Block, account: Account) -> BlockValidator {
    BlockValidator {
        block,
        epochs: &LEDGER_CONSTANTS_STUB.epochs,
        work: &WORK_THRESHOLDS_STUB,
        block_exists: false,
        account,
        old_account_info: None,
        previous_block: None,
        pending_receive_info: None,
        any_pending_exists: false,
        source_block_exists: false,
        seconds_since_epoch: 123456,
    }
}
