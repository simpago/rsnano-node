mod ledger_stats;
mod network_stats;
mod parse_message_error;
pub use ledger_stats::LedgerStats;
pub use network_stats::*;

use rsnano_core::VoteSource;
use rsnano_ledger::BlockStatus;
use rsnano_messages::Message;

use super::DetailType;

impl From<BlockStatus> for DetailType {
    fn from(value: BlockStatus) -> Self {
        match value {
            BlockStatus::Progress => Self::Progress,
            BlockStatus::BadSignature => Self::BadSignature,
            BlockStatus::Old => Self::Old,
            BlockStatus::NegativeSpend => Self::NegativeSpend,
            BlockStatus::Fork => Self::Fork,
            BlockStatus::Unreceivable => Self::Unreceivable,
            BlockStatus::GapPrevious => Self::GapPrevious,
            BlockStatus::GapSource => Self::GapSource,
            BlockStatus::GapEpochOpenPending => Self::GapEpochOpenPending,
            BlockStatus::OpenedBurnAccount => Self::OpenedBurnAccount,
            BlockStatus::BalanceMismatch => Self::BalanceMismatch,
            BlockStatus::RepresentativeMismatch => Self::RepresentativeMismatch,
            BlockStatus::BlockPosition => Self::BlockPosition,
            BlockStatus::InsufficientWork => Self::InsufficientWork,
        }
    }
}

impl From<&Message> for DetailType {
    fn from(value: &Message) -> Self {
        value.message_type().into()
    }
}

impl From<VoteSource> for DetailType {
    fn from(value: VoteSource) -> Self {
        match value {
            VoteSource::Live => Self::Live,
            VoteSource::Rebroadcast => Self::Rebroadcast,
            VoteSource::Cache => Self::Cache,
        }
    }
}
