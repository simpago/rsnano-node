mod common_rules;
mod epoch_block_rules;
mod helpers;
mod open_block_rules;
mod receive_block_rules;
mod send_block_rules;
#[cfg(test)]
mod tests;

use super::BlockInsertInstructions;
use crate::BlockStatus;
use rsnano_core::{
    utils::UnixTimestamp, work::WorkThresholds, Account, AccountInfo, Block, Epochs, PendingInfo,
    SavedBlock,
};

/// Validates a single block before it gets inserted into the ledger
pub(crate) struct BlockValidator<'a> {
    pub block: &'a Block,
    pub epochs: &'a Epochs,
    pub work: &'a WorkThresholds,
    pub block_exists: bool,
    pub account: Account,
    pub previous_block: Option<SavedBlock>,
    pub old_account_info: Option<AccountInfo>,
    pub pending_receive_info: Option<PendingInfo>,
    pub any_pending_exists: bool,
    pub source_block_exists: bool,
    pub now: UnixTimestamp,
}

impl<'a> BlockValidator<'a> {
    pub(crate) fn validate(&self) -> Result<BlockInsertInstructions, BlockStatus> {
        self.epoch_block_pre_checks()?;
        self.ensure_block_does_not_exist_yet()?;
        self.ensure_valid_predecessor()?;
        self.ensure_valid_signature()?;
        self.ensure_block_is_not_for_burn_account()?;
        self.ensure_account_exists_for_none_open_block()?;
        self.ensure_no_double_account_open()?;
        self.ensure_previous_block_is_correct()?;
        self.ensure_open_block_has_link()?;
        self.ensure_no_reveive_balance_change_without_link()?;
        self.ensure_pending_receive_is_correct()?;
        self.ensure_sufficient_work()?;
        self.ensure_no_negative_amount_send()?;
        self.ensure_valid_epoch_block()?;

        Ok(self.create_instructions())
    }

    fn create_instructions(&self) -> BlockInsertInstructions {
        BlockInsertInstructions {
            account: self.account,
            old_account_info: self.old_account_info.clone().unwrap_or_default(),
            set_account_info: self.new_account_info(),
            delete_pending: self.delete_received_pending_info(),
            insert_pending: self.new_pending_info(),
            set_sideband: self.new_sideband(),
            is_epoch_block: self.is_epoch_block(),
        }
    }
}
