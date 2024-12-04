use super::BlockValidator;
use crate::BlockStatus;
use rsnano_core::PublicKey;

impl<'a> BlockValidator<'a> {
    pub(crate) fn ensure_block_does_not_exist_yet(&self) -> Result<(), BlockStatus> {
        if self.block_exists {
            Err(BlockStatus::Old)
        } else {
            Ok(())
        }
    }

    pub(crate) fn ensure_valid_signature(&self) -> Result<(), BlockStatus> {
        let result = if self.is_epoch_block() {
            self.epochs.validate_epoch_signature(self.block)
        } else {
            let pub_key: PublicKey = self.account.into();
            pub_key.verify(self.block.hash().as_bytes(), self.block.block_signature())
        };
        result.map_err(|_| BlockStatus::BadSignature)
    }

    pub(crate) fn ensure_account_exists_for_none_open_block(&self) -> Result<(), BlockStatus> {
        if !self.block.is_open() && self.is_new_account() {
            Err(BlockStatus::GapPrevious)
        } else {
            Ok(())
        }
    }

    pub(crate) fn ensure_previous_block_is_correct(&self) -> Result<(), BlockStatus> {
        self.ensure_previous_block_exists()?;
        self.ensure_previous_block_is_account_head()
    }

    fn ensure_previous_block_exists(&self) -> Result<(), BlockStatus> {
        if self.account_exists() && self.previous_block.is_none() {
            return Err(BlockStatus::GapPrevious);
        }

        if self.is_new_account() && !self.block.previous().is_zero() {
            return Err(BlockStatus::GapPrevious);
        }

        Ok(())
    }

    fn ensure_previous_block_is_account_head(&self) -> Result<(), BlockStatus> {
        if let Some(info) = &self.old_account_info {
            if self.block.previous() != info.head {
                return Err(BlockStatus::Fork);
            }
        }

        Ok(())
    }

    pub(crate) fn ensure_sufficient_work(&self) -> Result<(), BlockStatus> {
        if !self.work.is_valid_pow(self.block, &self.block_details()) {
            Err(BlockStatus::InsufficientWork)
        } else {
            Ok(())
        }
    }

    pub(crate) fn ensure_valid_predecessor(&self) -> Result<(), BlockStatus> {
        if self.block.previous().is_zero() {
            return Ok(());
        }

        let previous = self
            .previous_block
            .as_ref()
            .ok_or(BlockStatus::GapPrevious)?;

        if !self.block.valid_predecessor(previous.block_type()) {
            Err(BlockStatus::BlockPosition)
        } else {
            Ok(())
        }
    }
}
