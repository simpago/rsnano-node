use super::BlockValidator;
use crate::BlockStatus;
use rsnano_core::Block;

impl<'a> BlockValidator<'a> {
    /// If there's no link, the balance must remain the same, only the representative can change
    pub(crate) fn ensure_no_reveive_balance_change_without_link(&self) -> Result<(), BlockStatus> {
        if let Block::State(state) = self.block {
            if state.link().is_zero() && !self.amount_received().is_zero() {
                return Err(BlockStatus::BalanceMismatch);
            }
        }

        Ok(())
    }

    pub(crate) fn ensure_no_negative_amount_send(&self) -> Result<(), BlockStatus> {
        // Is this trying to spend a negative amount (Malicious)
        if let Block::LegacySend(send) = self.block {
            if self.previous_balance() < send.balance() {
                return Err(BlockStatus::NegativeSpend);
            };
        }

        Ok(())
    }
}
