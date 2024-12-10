use crate::stats::{DetailType, StatType, Stats};
use rsnano_core::{utils::ContainerInfo, Account, BlockHash};
use rsnano_nullable_clock::{SteadyClock, Timestamp};
use std::{
    cmp::max,
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    time::Duration,
};

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
            let start = range_size * max(i, 1);
            let end = if i == config.head_parallelism - 1 {
                max_account
            } else {
                start + range_size
            };
            heads.push_back(FrontierHead::new(start.into(), end.into()));
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
        for head in self.heads.ordered_by_timestamp() {
            if head.requests < self.config.consideration_count || head.timestamp < cutoff {
                debug_assert!(head.next.number() >= head.start.number());
                debug_assert!(head.next.number() < head.end.number());

                self.stats.inc(
                    StatType::BootstrapAscendingFrontiers,
                    if head.requests < self.config.consideration_count {
                        DetailType::NextByRequests
                    } else {
                        DetailType::NextByTimestamp
                    },
                );

                next_account = head.next;
                break;
            }
        }

        if next_account.is_zero() {
            self.stats
                .inc(StatType::BootstrapAscendingFrontiers, DetailType::NextNone);
        } else {
            self.heads.modify(&next_account, |head| {
                head.requests += 1;
                head.timestamp = self.clock.now()
            });
        }

        next_account
    }

    pub fn process(&mut self, start: Account, response: Vec<(Account, BlockHash)>) -> bool {
        debug_assert!(response
            .iter()
            .all(|(acc, _)| acc.number() >= start.number()));

        self.stats
            .inc(StatType::BootstrapAscendingFrontiers, DetailType::Process);

        // Find the first head with head.start <= start
        let it = self.heads.find_first_less_than_or_equal_to(start).unwrap();

        let mut done = false;
        self.heads.modify(&it, |entry| {
            entry.completed += 1;

            for (account, _) in &response {
                // Only consider candidates that actually advance the current frontier
                if account.number() > entry.next.number() {
                    entry.candidates.insert(*account);
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
                    .inc(StatType::BootstrapAscendingFrontiers, DetailType::DoneEmpty);
                entry.candidates.insert(entry.end);
            }

            // Check if done
            if entry.completed >= self.config.consideration_count && !entry.candidates.is_empty() {
                self.stats
                    .inc(StatType::BootstrapAscendingFrontiers, DetailType::Done);

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
                        .inc(StatType::BootstrapAscendingFrontiers, DetailType::DoneRange);
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

struct FrontierHead {
    // The range of accounts to scan is [start, end)
    start: Account,
    end: Account,

    next: Account,
    candidates: BTreeSet<Account>,

    requests: usize,
    completed: usize,
    timestamp: Timestamp,

    /// Total number of accounts processed
    processed: usize,
}

impl FrontierHead {
    pub fn new(start: Account, end: Account) -> Self {
        Self {
            start,
            end,
            next: start,
            candidates: Default::default(),
            requests: 0,
            completed: 0,
            timestamp: Timestamp::default(),
            processed: 0,
        }
    }
}

#[derive(Default)]
struct OrderedHeads {
    sequenced: Vec<Account>,
    by_start: BTreeMap<Account, FrontierHead>,
    by_timestamp: BTreeMap<Timestamp, Vec<Account>>,
}

impl OrderedHeads {
    pub fn push_back(&mut self, head: FrontierHead) {
        let start = head.start;
        let timestamp = head.timestamp;
        let mut inserted = true;
        self.by_start
            .entry(start)
            .and_modify(|_| inserted = false)
            .or_insert(head);

        if !inserted {
            return;
        }
        self.sequenced.push(start);
        self.by_timestamp.entry(timestamp).or_default().push(start);
    }

    pub fn ordered_by_timestamp(&self) -> impl Iterator<Item = &FrontierHead> {
        self.by_timestamp
            .values()
            .flatten()
            .map(|start| self.by_start.get(start).unwrap())
    }

    pub fn modify<F>(&mut self, start: &Account, mut f: F)
    where
        F: FnMut(&mut FrontierHead),
    {
        if let Some(head) = self.by_start.get_mut(start) {
            let old_timestamp = head.timestamp;
            f(head);
            if head.timestamp != old_timestamp {
                let accounts = self.by_timestamp.get_mut(&old_timestamp).unwrap();
                if accounts.is_empty() {
                    self.by_timestamp.remove(&old_timestamp);
                } else {
                    accounts.retain(|a| a != start);
                }
                self.by_timestamp
                    .entry(head.timestamp)
                    .or_default()
                    .push(*start);
            }
        }
    }

    fn find_first_less_than_or_equal_to(&self, account: Account) -> Option<Account> {
        self.by_start
            .range(..=account)
            .last()
            .map(|(start, _)| *start)
    }

    pub fn iter(&self) -> impl Iterator<Item = &FrontierHead> {
        self.by_start.values()
    }

    pub fn len(&self) -> usize {
        self.sequenced.len()
    }
}
