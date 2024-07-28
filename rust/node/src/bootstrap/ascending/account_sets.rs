use super::{
    ordered_blocking::{BlockingEntry, OrderedBlocking},
    ordered_priorities::OrderedPriorities,
};
use crate::{
    bootstrap::ascending::ordered_priorities::PriorityEntry,
    config::AccountSetsConfig,
    stats::{DetailType, StatType, Stats},
};
use ordered_float::OrderedFloat;
use rand::{
    distributions::{Distribution, WeightedIndex},
    thread_rng, RngCore,
};
use rsnano_core::{
    utils::{ContainerInfo, ContainerInfoComponent},
    Account, BlockHash,
};
use std::{
    cmp::min,
    sync::Arc,
    time::{Duration, Instant},
};

/// This struct tracks accounts various account sets which are shared among the multiple bootstrap threads
pub(crate) struct AccountSets {
    stats: Arc<Stats>,
    config: AccountSetsConfig,
    priorities: OrderedPriorities,
    blocking: OrderedBlocking,
}

impl AccountSets {
    pub const PRIORITY_INITIAL: OrderedFloat<f32> = OrderedFloat(8.0);
    pub const PRIORITY_INCREASE: OrderedFloat<f32> = OrderedFloat(2.0);
    pub const PRIORITY_DECREASE: OrderedFloat<f32> = OrderedFloat(0.5);
    pub const PRIORITY_MAX: OrderedFloat<f32> = OrderedFloat(32.0);
    pub const PRIORITY_CUTOFF: OrderedFloat<f32> = OrderedFloat(1.0);

    pub fn new(stats: Arc<Stats>, config: AccountSetsConfig) -> Self {
        Self {
            stats,
            config,
            priorities: Default::default(),
            blocking: Default::default(),
        }
    }

    pub fn priority_up(&mut self, account: &Account) {
        if !self.blocked(account) {
            self.stats
                .inc(StatType::BootstrapAscendingAccounts, DetailType::Prioritize);

            if !self.priorities.change_priority(account, |prio| {
                Some(min(prio * Self::PRIORITY_INCREASE, Self::PRIORITY_MAX))
            }) {
                self.priorities
                    .insert(PriorityEntry::new(*account, Self::PRIORITY_INITIAL));
                self.stats.inc(
                    StatType::BootstrapAscendingAccounts,
                    DetailType::PriorityInsert,
                );

                self.trim_overflow();
            }
        } else {
            self.stats.inc(
                StatType::BootstrapAscendingAccounts,
                DetailType::PrioritizeFailed,
            );
        }
    }

    pub fn priority_down(&mut self, account: &Account) {
        if !self.priorities.change_priority(account, |prio| {
            self.stats.inc(
                StatType::BootstrapAscendingAccounts,
                DetailType::Deprioritize,
            );

            let priority_new = prio - Self::PRIORITY_DECREASE;
            if priority_new <= Self::PRIORITY_CUTOFF {
                self.stats.inc(
                    StatType::BootstrapAscendingAccounts,
                    DetailType::PriorityEraseThreshold,
                );
                None
            } else {
                Some(priority_new)
            }
        }) {
            self.stats.inc(
                StatType::BootstrapAscendingAccounts,
                DetailType::DeprioritizeFailed,
            );
        }
    }

    pub fn block(&mut self, account: Account, dependency: BlockHash) {
        self.stats
            .inc(StatType::BootstrapAscendingAccounts, DetailType::Block);

        let entry = self.priorities.remove(&account).unwrap_or_default();
        self.stats.inc(
            StatType::BootstrapAscendingAccounts,
            DetailType::PriorityEraseBlock,
        );

        self.blocking.insert(BlockingEntry {
            account,
            dependency,
            original_entry: entry,
        });
        self.stats.inc(
            StatType::BootstrapAscendingAccounts,
            DetailType::BlockingInsert,
        );

        self.trim_overflow();
    }

    pub fn unblock(&mut self, account: Account, hash: Option<BlockHash>) {
        // Unblock only if the dependency is fulfilled

        if let Some(existing) = self.blocking.get(&account) {
            let hash_matches = if let Some(hash) = hash {
                hash == existing.dependency
            } else {
                true
            };

            if hash_matches {
                self.stats
                    .inc(StatType::BootstrapAscendingAccounts, DetailType::Unblock);

                debug_assert!(!self.priorities.contains(&account));
                if !existing.original_entry.account.is_zero() {
                    debug_assert!(existing.original_entry.account == account);
                    self.priorities.insert(existing.original_entry.clone());
                } else {
                    self.priorities
                        .insert(PriorityEntry::new(account, Self::PRIORITY_INITIAL));
                }
                self.blocking.remove(&account);

                self.trim_overflow();
                return;
            }
        }
        self.stats.inc(
            StatType::BootstrapAscendingAccounts,
            DetailType::UnblockFailed,
        );
    }

    pub fn timestamp(&mut self, account: &Account, reset: bool) {
        let tstamp = if reset { None } else { Some(Instant::now()) };
        self.priorities.change_timestamp(account, tstamp);
    }

    fn check_timestamp(&self, account: &Account) -> bool {
        if let Some(entry) = self.priorities.get(account) {
            if entry
                .timestamp
                .map(|i| i.elapsed())
                .unwrap_or(Duration::MAX)
                < self.config.cooldown
            {
                return false;
            }
        }

        true
    }

    fn trim_overflow(&mut self) {
        if self.priorities.len() > self.config.priorities_max {
            self.priorities.pop_lowest_priority();
            self.stats.inc(
                StatType::BootstrapAscendingAccounts,
                DetailType::PriorityEraseOverflow,
            );
        }
        if self.blocking.len() > self.config.blocking_max {
            // Evict the lowest priority entry
            self.blocking.pop_lowest_priority();

            self.stats.inc(
                StatType::BootstrapAscendingAccounts,
                DetailType::BlockingEraseOverflow,
            );
        }
    }

    pub fn next(&self) -> Account {
        if self.priorities.is_empty() {
            return Account::zero();
        }

        let mut weights: Vec<f32> = Vec::new();
        let mut candidates: Vec<Account> = Vec::new();
        //
        let mut iterations = 0;
        while candidates.len() < self.config.consideration_count
            && iterations < self.config.consideration_count * 10
        {
            iterations += 1;
            debug_assert_eq!(candidates.len(), weights.len());

            // Use a dedicated, uniformly distributed field for sampling to avoid problematic corner case when accounts in the queue are very close together
            let search = thread_rng().next_u64();
            let entry = self.priorities.wrapping_lower_bound(search).unwrap();

            if self.check_timestamp(&entry.account) {
                candidates.push(entry.account);
                weights.push(*entry.priority);
            }
        }

        if candidates.is_empty() {
            return Account::zero(); // All sampled accounts are busy
        }

        let dist = WeightedIndex::new(weights).unwrap();
        let selection = dist.sample(&mut thread_rng());
        candidates[selection]
    }

    fn blocked(&self, account: &Account) -> bool {
        self.blocking.contains(account)
    }

    pub fn priority_len(&self) -> usize {
        self.priorities.len()
    }

    pub fn blocked_len(&self) -> usize {
        self.blocking.len()
    }

    #[allow(dead_code)]
    fn priority(&self, account: &Account) -> f32 {
        if self.blocked(account) {
            return 0.0;
        }

        if let Some(existing) = self.priorities.get(account) {
            *existing.priority
        } else {
            *Self::PRIORITY_CUTOFF
        }
    }

    pub fn collect_container_info(&self, name: impl Into<String>) -> ContainerInfoComponent {
        ContainerInfoComponent::Composite(
            name.into(),
            vec![
                ContainerInfoComponent::Leaf(ContainerInfo {
                    name: "priorities".to_string(),
                    count: self.priorities.len(),
                    sizeof_element: OrderedPriorities::ELEMENT_SIZE,
                }),
                ContainerInfoComponent::Leaf(ContainerInfo {
                    name: "blocking".to_string(),
                    count: self.blocking.len(),
                    sizeof_element: OrderedBlocking::ELEMENT_SIZE,
                }),
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_blocked() {
        fixture(|sets| {
            assert_eq!(sets.blocked(&Account::from(1)), false);
        });
    }

    #[test]
    fn block() {
        fixture(|sets| {
            let account = Account::from(1);
            let hash = BlockHash::from(2);

            sets.block(account, hash);

            assert!(sets.blocked(&account));
            assert_eq!(sets.priority(&account), 0f32);
        });
    }

    #[test]
    fn unblock() {
        fixture(|sets| {
            let account = Account::from(1);
            let hash = BlockHash::from(2);

            sets.block(account, hash);
            sets.unblock(account, None);

            assert_eq!(sets.blocked(&account), false);
        });
    }

    #[test]
    fn priority_base() {
        fixture(|sets| {
            assert_eq!(sets.priority(&Account::from(1)), 1f32);
        });
    }

    // When account is unblocked, check that it retains it former priority
    #[test]
    fn priority_unblock_keep() {
        fixture(|sets| {
            let account = Account::from(1);
            let hash = BlockHash::from(2);

            sets.priority_up(&account);
            sets.priority_up(&account);

            sets.block(account, hash);
            sets.unblock(account, None);

            assert_eq!(sets.priority(&account), 16f32);
        });
    }

    #[test]
    fn priority_up_down() {
        fixture(|sets| {
            let account = Account::from(1);

            sets.priority_up(&account);
            assert_eq!(sets.priority(&account), *AccountSets::PRIORITY_INITIAL);

            sets.priority_down(&account);
            assert_eq!(sets.priority(&account), 7.5f32);
        });
    }

    // Check that priority downward saturates to 1.0f
    #[test]
    fn priority_down_saturates() {
        fixture(|sets| {
            let account = Account::from(1);

            sets.priority_down(&account);
            assert_eq!(sets.priority(&account), 1f32);
        });
    }

    // Ensure priority value is bounded
    #[test]
    fn saturate_priority() {
        fixture(|sets| {
            let account = Account::from(1);

            for _ in 0..10 {
                sets.priority_up(&account);
            }
            assert_eq!(sets.priority(&account), *AccountSets::PRIORITY_MAX);
        });
    }

    fn fixture(mut f: impl FnMut(&mut AccountSets)) {
        let stats = Arc::new(Stats::default());
        let config = AccountSetsConfig::default();
        let mut sets = AccountSets::new(stats, config);
        f(&mut sets);
    }
}
