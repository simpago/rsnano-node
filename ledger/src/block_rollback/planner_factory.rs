use super::rollback_planner::RollbackPlanner;
use crate::Ledger;
use rsnano_core::{
    utils::UnixTimestamp, Account, AccountInfo, Block, BlockHash, ConfirmationHeightInfo,
    PendingInfo, PendingKey, PublicKey, SavedBlock,
};
use rsnano_store_lmdb::Transaction;

pub(crate) struct RollbackPlannerFactory<'a> {
    ledger: &'a Ledger,
    txn: &'a dyn Transaction,
    head_block: &'a SavedBlock,
}

impl<'a> RollbackPlannerFactory<'a> {
    pub(crate) fn new(
        ledger: &'a Ledger,
        txn: &'a dyn Transaction,
        head_block: &'a SavedBlock,
    ) -> Self {
        Self {
            ledger,
            txn,
            head_block,
        }
    }

    pub(crate) fn create_planner(&self) -> anyhow::Result<RollbackPlanner<'a>> {
        let account = self.get_account(self.head_block)?;
        let planner = RollbackPlanner {
            epochs: &self.ledger.constants.epochs,
            head_block: self.head_block.clone(),
            account,
            current_account_info: self.load_account(&account),
            previous_representative: self.get_previous_representative()?,
            previous: self.load_previous_block()?,
            linked_account: self.load_linked_account(),
            pending_receive: self.load_pending_receive(),
            latest_block_for_destination: self.latest_block_for_destination(),
            confirmation_height: self.account_confirmation_height(),
            now: UnixTimestamp::now().as_u64(),
        };
        Ok(planner)
    }

    fn latest_block_for_destination(&self) -> Option<BlockHash> {
        self.ledger
            .any()
            .account_head(self.txn, &self.head_block.destination_or_link())
    }

    fn load_pending_receive(&self) -> Option<PendingInfo> {
        self.ledger.any().get_pending(
            self.txn,
            &PendingKey::new(
                self.head_block.destination_or_link(),
                self.head_block.hash(),
            ),
        )
    }

    fn load_linked_account(&self) -> Account {
        self.ledger
            .any()
            .block_account(self.txn, &self.head_block.source_or_link())
            .unwrap_or_default()
    }

    fn load_previous_block(&self) -> anyhow::Result<Option<SavedBlock>> {
        let previous = self.head_block.previous();
        Ok(if previous.is_zero() {
            None
        } else {
            Some(self.load_block(&previous)?)
        })
    }

    fn account_confirmation_height(&self) -> ConfirmationHeightInfo {
        self.ledger
            .store
            .confirmation_height
            .get(self.txn, &self.head_block.account())
            .unwrap_or_default()
    }

    fn get_account(&self, block: &Block) -> anyhow::Result<Account> {
        self.ledger
            .any()
            .block_account(self.txn, &block.hash())
            .ok_or_else(|| anyhow!("account not found"))
    }

    fn load_account(&self, account: &Account) -> AccountInfo {
        self.ledger
            .account_info(self.txn, account)
            .unwrap_or_default()
    }

    fn load_block(&self, block_hash: &BlockHash) -> anyhow::Result<SavedBlock> {
        self.ledger
            .any()
            .get_block(self.txn, block_hash)
            .ok_or_else(|| anyhow!("block not found"))
    }

    fn get_previous_representative(&self) -> anyhow::Result<Option<PublicKey>> {
        let previous = self.head_block.previous();
        let rep_block_hash = if !previous.is_zero() {
            self.ledger.representative_block_hash(self.txn, &previous)
        } else {
            BlockHash::zero()
        };

        let previous_rep = if !rep_block_hash.is_zero() {
            let rep_block = self.load_block(&rep_block_hash)?;
            Some(rep_block.representative_field().unwrap_or_default())
        } else {
            None
        };
        Ok(previous_rep)
    }
}
