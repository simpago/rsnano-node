use crate::stats::{DetailType, StatType, Stats};
use rsnano_core::{Block, BlockHash, Root, SavedBlock};
use rsnano_ledger::Ledger;
use rsnano_store_lmdb::LmdbReadTransaction;

pub(super) struct RequestAggregatorImpl<'a> {
    ledger: &'a Ledger,
    stats: &'a Stats,
    tx: &'a LmdbReadTransaction,

    pub to_generate: Vec<SavedBlock>,
    pub to_generate_final: Vec<SavedBlock>,
}

impl<'a> RequestAggregatorImpl<'a> {
    pub fn new(ledger: &'a Ledger, stats: &'a Stats, tx: &'a LmdbReadTransaction) -> Self {
        Self {
            ledger,
            stats,
            tx,
            to_generate: Vec::new(),
            to_generate_final: Vec::new(),
        }
    }

    pub fn add_votes(&mut self, requests: &[(BlockHash, Root)]) {
        for (hash, root) in requests {
            // Ledger by hash
            let mut block = self.ledger.any().get_block(self.tx, hash);

            // Ledger by root
            if block.is_none() && !root.is_zero() {
                // Search for block root
                if let Some(successor) = self.ledger.any().block_successor(self.tx, &(*root).into())
                {
                    block = self.ledger.any().get_block(self.tx, &successor);
                }
            }

            let should_generate_final_vote = |block: &Block| {
                // Check if final vote is set for this block
                if let Some(final_hash) = self
                    .ledger
                    .store
                    .final_vote
                    .get(self.tx, &block.qualified_root())
                {
                    final_hash == block.hash()
                } else {
                    // If the final vote is not set, generate vote if the block is confirmed
                    self.ledger.confirmed().block_exists(self.tx, &block.hash())
                }
            };

            if let Some(block) = block {
                if should_generate_final_vote(&block) {
                    self.to_generate_final.push(block);
                    self.stats
                        .inc(StatType::Requests, DetailType::RequestsFinal);
                } else {
                    self.stats
                        .inc(StatType::Requests, DetailType::RequestsNonFinal);
                }
            } else {
                self.stats
                    .inc(StatType::Requests, DetailType::RequestsUnknown);
            }
        }
    }

    pub fn get_result(self) -> AggregateResult {
        AggregateResult {
            remaining_normal: self.to_generate,
            remaining_final: self.to_generate_final,
        }
    }
}

pub(super) struct AggregateResult {
    pub remaining_normal: Vec<SavedBlock>,
    pub remaining_final: Vec<SavedBlock>,
}
