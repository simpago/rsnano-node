use rsnano_core::{
    AccountInfo, Amount, Block, BlockBase, BlockDetails, BlockHash, BlockSideband, Epoch,
    PendingInfo, PendingKey, PublicKey, StateBlock,
};

use super::BlockValidator;

impl<'a> BlockValidator<'a> {
    pub(crate) fn account_exists(&self) -> bool {
        self.old_account_info.is_some()
    }

    pub(crate) fn is_new_account(&self) -> bool {
        self.old_account_info.is_none()
    }

    pub(crate) fn previous_balance(&self) -> Amount {
        self.previous_block
            .as_ref()
            .map(|b| b.balance())
            .unwrap_or_default()
    }

    pub(crate) fn is_send(&self) -> bool {
        match self.block {
            Block::LegacySend(_) => true,
            Block::State(state) => match &self.old_account_info {
                Some(info) => state.balance() < info.balance,
                None => false,
            },
            _ => false,
        }
    }

    pub(crate) fn is_receive(&self) -> bool {
        match self.block {
            Block::LegacyReceive(_) | Block::LegacyOpen(_) => true,
            Block::State(state_block) => {
                // receives from the epoch account are forbidden
                if self.has_epoch_link(state_block) {
                    return false;
                }

                match &self.old_account_info {
                    Some(info) => {
                        state_block.balance() >= info.balance && !state_block.link().is_zero()
                    }
                    None => true,
                }
            }
            _ => false,
        }
    }

    pub(crate) fn source_epoch(&self) -> Epoch {
        self.pending_receive_info
            .as_ref()
            .map(|p| p.epoch)
            .unwrap_or(Epoch::Epoch0)
    }

    pub(crate) fn amount_received(&self) -> Amount {
        match &self.block {
            Block::LegacyReceive(_) | Block::LegacyOpen(_) => self.pending_amount(),
            Block::State(state) => {
                let previous = self.previous_balance();
                if previous < state.balance() {
                    state.balance() - previous
                } else {
                    Amount::zero()
                }
            }
            _ => Amount::zero(),
        }
    }

    pub fn pending_amount(&self) -> Amount {
        self.pending_receive_info
            .as_ref()
            .map(|i| i.amount)
            .unwrap_or_default()
    }

    pub(crate) fn amount_sent(&self) -> Amount {
        if let Some(info) = &self.old_account_info {
            let balance = match self.block {
                Block::LegacySend(i) => Some(i.balance()),
                Block::State(i) => Some(i.balance()),
                _ => None,
            };
            if let Some(balance) = balance {
                if balance < info.balance {
                    return info.balance - balance;
                }
            }
        }
        Amount::zero()
    }

    pub(crate) fn new_balance(&self) -> Amount {
        self.old_balance() + self.amount_received() - self.amount_sent()
    }

    fn old_balance(&self) -> Amount {
        self.old_account_info
            .as_ref()
            .map(|i| i.balance)
            .unwrap_or_default()
    }

    pub(crate) fn has_epoch_link(&self, state_block: &StateBlock) -> bool {
        self.epochs.is_epoch_link(&state_block.link())
    }

    /// This check only makes sense after ensure_previous_block_exists_for_epoch_block_candidate,
    /// because we need the previous block for the balance change check!
    pub(crate) fn is_epoch_block(&self) -> bool {
        match self.block {
            Block::State(state_block) => {
                self.has_epoch_link(state_block) && self.previous_balance() == state_block.balance()
            }
            _ => false,
        }
    }

    pub(crate) fn block_epoch_version(&self) -> Epoch {
        match self.block {
            Block::State(state) => self.epochs.epoch(&state.link()).unwrap_or(Epoch::Invalid),
            _ => Epoch::Epoch0,
        }
    }

    pub(crate) fn epoch(&self) -> Epoch {
        if self.is_epoch_block() {
            self.block_epoch_version()
        } else {
            std::cmp::max(self.old_epoch_version(), self.source_epoch())
        }
    }

    fn old_epoch_version(&self) -> Epoch {
        self.old_account_info
            .as_ref()
            .map(|i| i.epoch)
            .unwrap_or(Epoch::Epoch0)
    }

    pub(crate) fn open_block(&self) -> BlockHash {
        match &self.old_account_info {
            Some(info) => info.open_block,
            None => self.block.hash(),
        }
    }

    pub(crate) fn new_representative(&self) -> PublicKey {
        self.block
            .representative_field()
            .unwrap_or(self.old_representative())
    }

    fn old_representative(&self) -> PublicKey {
        self.old_account_info
            .as_ref()
            .map(|x| x.representative)
            .unwrap_or_default()
    }

    pub(crate) fn amount(&self) -> Amount {
        let old_balance = self.old_balance();
        let new_balance = self.new_balance();

        if old_balance > new_balance {
            old_balance - new_balance
        } else {
            new_balance - old_balance
        }
    }

    pub(crate) fn delete_received_pending_info(&self) -> Option<PendingKey> {
        if self.is_receive() {
            Some(PendingKey::new(self.account, self.block.source_or_link()))
        } else {
            None
        }
    }

    pub(crate) fn new_pending_info(&self) -> Option<(PendingKey, PendingInfo)> {
        match self.block {
            Block::State(_) => {
                if self.is_send() {
                    let key = PendingKey::for_send_block(self.block);
                    let info = PendingInfo::new(self.account, self.amount(), self.epoch());
                    Some((key, info))
                } else {
                    None
                }
            }
            Block::LegacySend(send) => {
                let amount_sent = self.amount_sent();
                Some((
                    PendingKey::new(send.destination(), send.hash()),
                    PendingInfo::new(self.account, amount_sent, Epoch::Epoch0),
                ))
            }
            _ => None,
        }
    }

    pub(crate) fn new_sideband(&self) -> BlockSideband {
        BlockSideband::new(
            self.account,
            BlockHash::zero(),
            self.new_balance(),
            self.new_block_count(),
            self.now.as_u64(),
            self.block_details(),
            self.source_epoch(),
        )
    }

    pub(crate) fn new_account_info(&self) -> AccountInfo {
        AccountInfo {
            head: self.block.hash(),
            representative: self.new_representative(),
            open_block: self.open_block(),
            balance: self.new_balance(),
            modified: self.now.as_u64(),
            block_count: self.new_block_count(),
            epoch: self.epoch(),
        }
    }

    pub(crate) fn new_block_count(&self) -> u64 {
        self.old_account_info
            .as_ref()
            .map(|info| info.block_count)
            .unwrap_or_default()
            + 1
    }

    pub(crate) fn block_details(&self) -> BlockDetails {
        BlockDetails::new(
            self.epoch(),
            self.is_send(),
            self.is_receive(),
            self.is_epoch_block(),
        )
    }

    pub(crate) fn balance_changed(&self) -> bool {
        if let Some(info) = &self.old_account_info {
            self.new_balance() != info.balance
        } else {
            false
        }
    }
}
