use std::{
    cmp::max,
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use rsnano_core::{utils::ContainerInfo, Account, BlockHash};
use rsnano_nullable_clock::Timestamp;

use crate::stats::Stats;

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
}

impl FrontierScan {
    pub fn new(config: FrontierScanConfig, stats: Arc<Stats>) -> Self {
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

        Self {
            config,
            stats,
            heads,
        }
    }

    pub fn next(&self) -> Account {
        todo!()
    }

    pub fn process(&self, start: Account, response: Vec<(Account, BlockHash)>) -> bool {
        todo!()
    }

    pub fn container_info(&self) -> ContainerInfo {
        todo!()
    }
}

struct FrontierHead {
    // The range of accounts to scan is [start, end)
    start: Account,
    end: Account,

    next: Account,
    candidates: HashSet<Account>,

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
    by_start: HashMap<Account, FrontierHead>,
    by_timestamp: HashMap<Timestamp, Vec<Account>>,
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
}
