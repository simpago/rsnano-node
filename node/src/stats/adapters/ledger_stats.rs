use crate::stats::{DetailType, Direction, StatType, Stats};
use rsnano_core::{Block, BlockSubType};
use rsnano_ledger::LedgerObserver;
use std::sync::Arc;

pub struct LedgerStats {
    stats: Arc<Stats>,
}

impl LedgerStats {
    pub fn new(stats: Arc<Stats>) -> Self {
        Self { stats }
    }
}

impl LedgerObserver for LedgerStats {
    fn blocks_cemented(&self, cemented_count: u64) {
        self.stats.add_dir(
            StatType::ConfirmationHeight,
            DetailType::BlocksConfirmed,
            Direction::In,
            cemented_count,
        );
    }

    fn block_rolled_back(&self, block_type: BlockSubType) {
        self.stats.inc(StatType::Rollback, block_type.into());
    }

    fn block_rolled_back2(&self, block: &Block, is_epoch: bool) {
        self.stats
            .inc(StatType::Ledger, block_detail_type(block, is_epoch));
    }

    fn dependent_unconfirmed(&self) {
        self.stats.inc(
            StatType::ConfirmationHeight,
            DetailType::DependentUnconfirmed,
        );
    }
}

fn block_detail_type(block: &Block, is_epoch: bool) -> DetailType {
    match block {
        Block::LegacySend(_) => DetailType::Send,
        Block::LegacyReceive(_) => DetailType::Receive,
        Block::LegacyOpen(_) => DetailType::Open,
        Block::LegacyChange(_) => DetailType::Change,
        Block::State(_) => {
            if is_epoch {
                DetailType::EpochBlock
            } else {
                DetailType::StateBlock
            }
        }
    }
}

impl From<BlockSubType> for DetailType {
    fn from(block_type: BlockSubType) -> Self {
        match block_type {
            BlockSubType::Send => DetailType::Send,
            BlockSubType::Receive => DetailType::Receive,
            BlockSubType::Open => DetailType::Open,
            BlockSubType::Change => DetailType::Change,
            BlockSubType::Epoch => DetailType::EpochBlock,
        }
    }
}
