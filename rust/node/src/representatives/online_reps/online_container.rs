use rsnano_core::Account;
use rsnano_nullable_clock::Timestamp;
use std::{
    collections::{BTreeMap, HashMap},
    mem::size_of,
    time::Duration,
};

/// Collection of all representatives that are currently online
#[derive(Default)]
pub(super) struct OnlineContainer {
    by_time: BTreeMap<Timestamp, Vec<Account>>,
    by_account: HashMap<Account, Timestamp>,
}

impl OnlineContainer {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Account> {
        self.by_account.keys()
    }

    /// Returns `true` if it was a new insert and `false` if an entry for that account was already present
    pub fn insert(&mut self, rep: Account, now: Timestamp) -> bool {
        let new_insert = if let Some(time) = self.by_account.get_mut(&rep) {
            let old_time = *time;
            *time = now;

            let accounts_for_old_time = self.by_time.get_mut(&old_time).unwrap();
            if accounts_for_old_time.len() == 1 {
                self.by_time.remove(&old_time);
            } else {
                accounts_for_old_time.retain(|acc| acc != &rep);
            }
            self.by_time.entry(now).or_default().push(rep);

            false
        } else {
            self.by_account.insert(rep, now);
            self.by_time.entry(now).or_default().push(rep);
            true
        };

        new_insert
    }

    pub fn trim(&mut self, upper_bound: Timestamp) -> bool {
        let mut trimmed = false;

        while let Some((time, _)) = self.by_time.first_key_value() {
            if *time >= upper_bound {
                break;
            }

            let (_, accounts) = self.by_time.pop_first().unwrap();
            for account in accounts {
                self.by_account.remove(&account);
            }

            trimmed = true;
        }

        trimmed
    }

    pub fn len(&self) -> usize {
        self.by_account.len()
    }

    pub const ELEMENT_SIZE: usize =
        size_of::<(Duration, Vec<Account>)>() + size_of::<(Account, Duration)>();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_container() {
        let container = OnlineContainer::new();
        assert_eq!(container.len(), 0);
        assert_eq!(container.iter().count(), 0);
    }

    #[test]
    fn insert_one_rep() {
        let mut container = OnlineContainer::new();

        let new_insert = container.insert(Account::from(1), Timestamp::new_test_instance());

        assert_eq!(container.len(), 1);
        assert_eq!(container.iter().count(), 1);
        assert_eq!(container.iter().next().unwrap(), &Account::from(1));
        assert_eq!(new_insert, true);
    }

    #[test]
    fn insert_two_reps() {
        let mut container = OnlineContainer::new();

        let now = Timestamp::new_test_instance();
        let new_insert_a = container.insert(Account::from(1), now);
        let new_insert_b = container.insert(Account::from(2), now + Duration::from_secs(1));

        assert_eq!(container.len(), 2);
        assert_eq!(container.iter().count(), 2);
        assert_eq!(new_insert_a, true);
        assert_eq!(new_insert_b, true);
    }

    #[test]
    fn insert_same_rep_twice_with_same_time() {
        let mut container = OnlineContainer::new();

        let now = Timestamp::new_test_instance();
        let new_insert_a = container.insert(Account::from(1), now);
        let new_insert_b = container.insert(Account::from(1), now);

        assert_eq!(container.len(), 1);
        assert_eq!(container.iter().count(), 1);
        assert_eq!(new_insert_a, true);
        assert_eq!(new_insert_b, false);
    }

    #[test]
    fn insert_same_rep_twice_with_different_time() {
        let mut container = OnlineContainer::new();

        let now = Timestamp::new_test_instance();
        let new_insert_a = container.insert(Account::from(1), now);
        let new_insert_b = container.insert(Account::from(1), now + Duration::from_secs(1));

        assert_eq!(container.len(), 1);
        assert_eq!(container.iter().count(), 1);
        assert_eq!(new_insert_a, true);
        assert_eq!(new_insert_b, false);
        assert_eq!(container.by_time.len(), 1);
    }

    #[test]
    fn trimming_empty_container_does_nothing() {
        let mut container = OnlineContainer::new();
        let now = Timestamp::new_test_instance();
        assert_eq!(container.trim(now), false);
    }

    #[test]
    fn dont_trim_if_upper_bound_not_reached() {
        let mut container = OnlineContainer::new();
        let now = Timestamp::new_test_instance();
        container.insert(Account::from(1), now);
        assert_eq!(container.trim(now), false);
    }

    #[test]
    fn trim_if_upper_bound_reached() {
        let mut container = OnlineContainer::new();
        let now = Timestamp::new_test_instance();
        container.insert(Account::from(1), now);
        assert_eq!(container.trim(now + Duration::from_millis(1)), true);
        assert_eq!(container.len(), 0);
    }

    #[test]
    fn trim_multiple_entries() {
        let mut container = OnlineContainer::new();

        let now = Timestamp::new_test_instance();
        container.insert(Account::from(1), now);
        container.insert(Account::from(2), now);
        container.insert(Account::from(3), now + Duration::from_secs(1));
        container.insert(Account::from(4), now + Duration::from_secs(2));

        assert_eq!(container.trim(now + Duration::from_millis(1500)), true);
        assert_eq!(container.len(), 1);
        assert_eq!(container.iter().next().unwrap(), &Account::from(4));
        assert_eq!(container.by_time.len(), 1);
    }
}
