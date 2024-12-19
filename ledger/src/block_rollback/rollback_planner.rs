use num_traits::AsPrimitive;
use rsnano_core::{
    utils::UnixTimestamp, Account, AccountInfo, Amount, BlockHash, BlockSubType,
    ConfirmationHeightInfo, Epoch, Epochs, PendingInfo, PendingKey, PublicKey, SavedBlock,
};

pub(crate) enum RollbackStep {
    RollBackBlock(RollbackInstructions),
    /// the given dependent block has to be rolled back first
    RequestDependencyRollback(BlockHash),
}

/// Describes how to roll back a block
pub(crate) struct RollbackInstructions {
    pub block_hash: BlockHash,
    pub block_sub_type: BlockSubType,
    pub account: Account,
    pub remove_pending: Option<PendingKey>,
    pub add_pending: Option<(PendingKey, PendingInfo)>,
    pub set_account_info: AccountInfo,
    pub old_account_info: AccountInfo,
    pub clear_successor: Option<BlockHash>,
    pub new_balance: Amount,
    pub new_representative: Option<PublicKey>,
}

/// Create RollbackInstructions for a given block
pub(crate) struct RollbackPlanner<'a> {
    pub epochs: &'a Epochs,
    pub head_block: SavedBlock,
    pub account: Account,
    pub current_account_info: AccountInfo,
    pub previous_representative: Option<PublicKey>,
    pub previous: Option<SavedBlock>,
    pub linked_account: Account,
    pub pending_receive: Option<PendingInfo>,
    pub latest_block_for_destination: Option<BlockHash>,
    pub confirmation_height: ConfirmationHeightInfo,
    pub now: UnixTimestamp,
}

impl<'a> RollbackPlanner<'a> {
    pub(crate) fn roll_back_head_block(&self) -> anyhow::Result<RollbackStep> {
        self.ensure_block_is_not_confirmed()?;
        let block_sub_type = self.block_sub_type();

        if block_sub_type == BlockSubType::Send {
            if let Some(step) = self.roll_back_destination_account_if_send_block_is_received()? {
                return Ok(step);
            }
        }

        let instructions = RollbackInstructions {
            block_hash: self.head_block.hash(),
            account: self.account,
            old_account_info: self.current_account_info.clone(),
            new_representative: self.previous_representative,
            block_sub_type,
            remove_pending: self.remove_pending(),
            add_pending: self.add_pending(),
            set_account_info: self.previous_account_info(),
            clear_successor: self.previous.as_ref().map(|b| b.hash()),
            new_balance: self.previous_balance(),
        };

        Ok(RollbackStep::RollBackBlock(instructions))
    }

    fn ensure_block_is_not_confirmed(&self) -> anyhow::Result<()> {
        if self.head_block.height() <= self.confirmation_height.height {
            bail!("Only unconfirmed blocks can be rolled back")
        }

        Ok(())
    }

    fn roll_back_destination_account_if_send_block_is_received(
        &self,
    ) -> anyhow::Result<Option<RollbackStep>> {
        if self.pending_receive.is_some() {
            return Ok(None);
        }

        let latest_destination_block = self
            .latest_block_for_destination
            .ok_or_else(|| anyhow!("no latest block for destination"))?;

        Ok(Some(RollbackStep::RequestDependencyRollback(
            latest_destination_block,
        )))
    }

    fn add_pending(&self) -> Option<(PendingKey, PendingInfo)> {
        match self.block_sub_type() {
            BlockSubType::Open | BlockSubType::Receive => {
                let source_hash = self.head_block.source_or_link();
                // Pending account entry can be incorrect if source block was pruned. But it's not affecting correct ledger processing
                Some((
                    PendingKey::new(self.account, source_hash),
                    PendingInfo::new(
                        self.linked_account,
                        self.current_account_info.balance - self.previous_balance(),
                        self.head_block.source_epoch(),
                    ),
                ))
            }
            _ => None,
        }
    }

    fn remove_pending(&self) -> Option<PendingKey> {
        if self.block_sub_type() == BlockSubType::Send {
            Some(PendingKey::new(
                self.head_block.destination_or_link(),
                self.head_block.hash(),
            ))
        } else {
            None
        }
    }

    fn block_sub_type(&self) -> BlockSubType {
        if self.current_account_info.balance < self.previous_balance() {
            BlockSubType::Send
        } else if self.current_account_info.balance > self.previous_balance() {
            if self.head_block.is_open() {
                BlockSubType::Open
            } else {
                BlockSubType::Receive
            }
        } else if self
            .epochs
            .is_epoch_link(&self.head_block.link_field().unwrap_or_default())
        {
            BlockSubType::Epoch
        } else {
            BlockSubType::Change
        }
    }

    fn previous_account_info(&self) -> AccountInfo {
        if self.head_block.previous().is_zero() {
            Default::default()
        } else {
            AccountInfo {
                head: self.head_block.previous(),
                representative: self.previous_representative(),
                open_block: self.current_account_info.open_block,
                balance: self.previous_balance(),
                modified: self.now,
                block_count: self.current_account_info.block_count - 1,
                epoch: self.previous_epoch(),
            }
        }
    }

    fn previous_representative(&self) -> PublicKey {
        self.previous_representative
            .unwrap_or(self.current_account_info.representative)
    }

    fn previous_epoch(&self) -> Epoch {
        match &self.previous {
            Some(previous) => previous.epoch(),
            None => Epoch::Epoch0,
        }
    }

    fn previous_balance(&self) -> Amount {
        match &self.previous {
            Some(previous) => previous.balance(),
            None => Amount::zero(),
        }
    }
}
