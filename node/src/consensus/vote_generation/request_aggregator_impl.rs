use crate::stats::{DetailType, StatType, Stats};
use rsnano_core::{BlockHash, Root, SavedBlock};
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
            let mut generate_final_vote = false;
            let mut block = None;

            // 2. Final votes
            let final_vote_hashes = self.ledger.store.final_vote.get(self.tx, *root);
            if !final_vote_hashes.is_empty() {
                generate_final_vote = true;
                block = self.ledger.any().get_block(self.tx, &final_vote_hashes[0]);
                // Allow same root vote
                if let Some(b) = &block {
                    if final_vote_hashes.len() > 1 {
                        // WTF? This shouldn't be done like this
                        self.to_generate_final.push(b.clone());
                        block = self.ledger.any().get_block(self.tx, &final_vote_hashes[1]);
                        debug_assert!(final_vote_hashes.len() == 2);
                    }
                }
            }

            // 4. Ledger by hash
            if block.is_none() {
                block = self.ledger.any().get_block(self.tx, hash);
                // Confirmation status. Generate final votes for confirmed
                if let Some(b) = &block {
                    let conf_height = self
                        .ledger
                        .store
                        .confirmation_height
                        .get(self.tx, &b.account())
                        .unwrap_or_default();
                    generate_final_vote = conf_height.height >= b.height();
                }
            }

            // 5. Ledger by root
            if block.is_none() && !root.is_zero() {
                // Search for block root
                let successor = self.ledger.any().block_successor(self.tx, &(*root).into());
                if let Some(successor) = successor {
                    let successor_block = self.ledger.any().get_block(self.tx, &successor).unwrap();
                    block = Some(successor_block);

                    // Confirmation status. Generate final votes for confirmed successor
                    if let Some(b) = &block {
                        let conf_height = self
                            .ledger
                            .store
                            .confirmation_height
                            .get(self.tx, &b.account())
                            .unwrap_or_default();
                        generate_final_vote = conf_height.height >= b.height();
                    }
                }
            }

            if let Some(block) = block {
                if generate_final_vote {
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
