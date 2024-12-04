use super::BlockValidator;
use crate::BlockStatus;
use rsnano_core::{Block, Epoch};

impl<'a> BlockValidator<'a> {
    pub fn ensure_pending_receive_is_correct(&self) -> Result<(), BlockStatus> {
        self.ensure_source_block_exists()?;
        self.ensure_receive_block_receives_pending_amount()?;
        self.ensure_legacy_source_is_epoch_0()
    }

    fn ensure_source_block_exists(&self) -> Result<(), BlockStatus> {
        if self.is_receive() && !self.source_block_exists {
            Err(BlockStatus::GapSource)
        } else {
            Ok(())
        }
    }

    fn ensure_receive_block_receives_pending_amount(&self) -> Result<(), BlockStatus> {
        if self.is_receive() {
            match &self.pending_receive_info {
                Some(pending) => {
                    if self.amount_received() != pending.amount {
                        return Err(BlockStatus::BalanceMismatch);
                    }
                }
                None => {
                    return Err(BlockStatus::Unreceivable);
                }
            };
        }

        Ok(())
    }

    fn ensure_legacy_source_is_epoch_0(&self) -> Result<(), BlockStatus> {
        let is_legacy_receive =
            matches!(self.block, Block::LegacyReceive(_) | Block::LegacyOpen(_));

        if is_legacy_receive
            && self
                .pending_receive_info
                .as_ref()
                .map(|x| x.epoch)
                .unwrap_or_default()
                != Epoch::Epoch0
        {
            Err(BlockStatus::Unreceivable)
        } else {
            Ok(())
        }
    }
}
