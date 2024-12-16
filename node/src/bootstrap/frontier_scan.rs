use super::ordered_heads::OrderedHeads;
use crate::{
    bootstrap::ordered_heads::FrontierHead,
    stats::{DetailType, StatType, Stats},
};
use primitive_types::U256;
use rsnano_core::{utils::ContainerInfo, Account, Frontier};
use rsnano_nullable_clock::{SteadyClock, Timestamp};
use std::{sync::Arc, time::Duration};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FrontierScanConfig {
    pub head_parallelism: usize,
    pub consideration_count: usize,
    pub candidates: usize,
    pub cooldown: Duration,
    pub max_pending: usize,
}

impl Default for FrontierScanConfig {
    fn default() -> Self {
        Self {
            head_parallelism: 128,
            consideration_count: 4,
            candidates: 1000,
            cooldown: Duration::from_secs(5),
            max_pending: 16,
        }
    }
}

/// Frontier scan divides the account space into ranges and scans each range for
/// outdated frontiers in parallel.
/// This class is used to track the progress of each range.
pub struct FrontierScan {
    config: FrontierScanConfig,
    stats: Arc<Stats>,
    heads: OrderedHeads,
    clock: Arc<SteadyClock>,
}

impl FrontierScan {
    pub fn new(config: FrontierScanConfig, stats: Arc<Stats>, clock: Arc<SteadyClock>) -> Self {
        // Divide nano::account numeric range into consecutive and equal ranges
        let max_account = Account::MAX.number();
        let range_size = max_account / config.head_parallelism;
        let mut heads = OrderedHeads::default();

        for i in 0..config.head_parallelism {
            // Start at 1 to avoid the burn account
            let start = if i == 0 {
                U256::from(1)
            } else {
                range_size * i
            };
            let end = if i == config.head_parallelism - 1 {
                max_account
            } else {
                start + range_size
            };
            heads.push_back(FrontierHead::new(start, end));
        }

        assert!(!heads.len() > 0);

        Self {
            config,
            stats,
            heads,
            clock,
        }
    }

    pub fn next(&mut self) -> Account {
        let cutoff = self.clock.now() - self.config.cooldown;
        let mut next_account = Account::zero();
        let mut it = Account::zero();
        for head in self.heads.ordered_by_timestamp() {
            if head.requests < self.config.consideration_count || head.timestamp < cutoff {
                debug_assert!(head.next.number() >= head.start.number());
                debug_assert!(head.next.number() < head.end.number());

                self.stats.inc(
                    StatType::BootstrapFrontierScan,
                    if head.requests < self.config.consideration_count {
                        DetailType::NextByRequests
                    } else {
                        DetailType::NextByTimestamp
                    },
                );

                next_account = head.next;
                it = head.start;
                break;
            }
        }

        if next_account.is_zero() {
            self.stats
                .inc(StatType::BootstrapFrontierScan, DetailType::NextNone);
        } else {
            self.heads.modify(&it, |head| {
                head.requests += 1;
                head.timestamp = self.clock.now()
            });
        }

        next_account
    }

    pub fn process(&mut self, start: Account, response: &[Frontier]) -> bool {
        debug_assert!(response
            .iter()
            .all(|f| f.account.number() >= start.number()));

        self.stats
            .inc(StatType::BootstrapFrontierScan, DetailType::Process);

        // Find the first head with head.start <= start
        let it = self.heads.find_first_less_than_or_equal_to(start).unwrap();

        let mut done = false;
        self.heads.modify(&it, |entry| {
            entry.completed += 1;

            for frontier in response {
                // Only consider candidates that actually advance the current frontier
                if frontier.account.number() > entry.next.number() {
                    entry.candidates.insert(frontier.account);
                }
            }

            // Trim the candidates
            while entry.candidates.len() > self.config.candidates {
                entry.candidates.pop_last();
            }

            // Special case for the last frontier head that won't receive larger than max frontier
            if entry.completed >= self.config.consideration_count * 2 && entry.candidates.is_empty()
            {
                self.stats
                    .inc(StatType::BootstrapFrontierScan, DetailType::DoneEmpty);
                entry.candidates.insert(entry.end);
            }

            // Check if done
            if entry.completed >= self.config.consideration_count && !entry.candidates.is_empty() {
                self.stats
                    .inc(StatType::BootstrapFrontierScan, DetailType::Done);

                // Take the last candidate as the next frontier
                assert!(!entry.candidates.is_empty());
                let last = entry.candidates.last().unwrap();
                debug_assert!(entry.next.number() < last.number());
                entry.next = *last;
                entry.processed += entry.candidates.len();
                entry.candidates.clear();
                entry.requests = 0;
                entry.completed = 0;
                entry.timestamp = Timestamp::default();

                // Bound the search range
                if entry.next.number() >= entry.end.number() {
                    self.stats
                        .inc(StatType::BootstrapFrontierScan, DetailType::DoneRange);
                    entry.next = entry.start;
                }

                done = true;
            }
        });

        done
    }

    pub fn container_info(&self) -> ContainerInfo {
        // TODO port the detailed container info from nano_node
        let total_processed = self.heads.iter().map(|i| i.processed).sum();
        [("total_processed", total_processed, 0)].into()
    }
}
