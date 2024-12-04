use crate::ledger_stats::LedgerStats;
use num_format::{Locale, ToFormattedString};

pub(crate) struct LedgerStatsViewModel<'a>(&'a LedgerStats);

impl<'a> LedgerStatsViewModel<'a> {
    pub(crate) fn new(stats: &'a LedgerStats) -> Self {
        Self(stats)
    }

    pub(crate) fn block_count(&self) -> String {
        self.0.total_blocks.to_formatted_string(&Locale::en)
    }

    pub(crate) fn cemented_count(&self) -> String {
        self.0.cemented_blocks.to_formatted_string(&Locale::en)
    }

    pub(crate) fn blocks_per_second(&self) -> String {
        self.0.blocks_per_second().to_formatted_string(&Locale::en)
    }

    pub(crate) fn confirmations_per_second(&self) -> String {
        self.0
            .confirmations_per_second()
            .to_formatted_string(&Locale::en)
    }
}
