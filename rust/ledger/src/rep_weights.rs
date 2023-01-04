use std::collections::HashMap;
use std::mem::size_of;
use std::sync::{Mutex, MutexGuard};

use rsnano_core::{Account, Amount};

#[derive(Debug)]
pub struct RepWeights {
    rep_amounts: Mutex<HashMap<Account, Amount>>,
}

impl RepWeights {
    pub fn new() -> Self {
        RepWeights {
            rep_amounts: Mutex::new(HashMap::new()),
        }
    }

    fn get(&self, guard: &MutexGuard<HashMap<Account, Amount>>, account: &Account) -> Amount {
        guard.get(account).cloned().unwrap_or_default()
    }

    fn put(
        &self,
        guard: &mut MutexGuard<HashMap<Account, Amount>>,
        account: Account,
        representation: Amount,
    ) {
        guard.insert(account, representation);
    }

    pub fn get_rep_amounts(&self) -> HashMap<Account, Amount> {
        self.rep_amounts.lock().unwrap().clone()
    }

    pub fn copy_from(&self, other: &RepWeights) {
        let mut guard_this = self.rep_amounts.lock().unwrap();
        let guard_other = other.rep_amounts.lock().unwrap();
        for (account, amount) in guard_other.iter() {
            let prev_amount = self.get(&guard_this, account);
            self.put(&mut guard_this, *account, prev_amount.wrapping_add(*amount));
        }
    }

    pub fn representation_add(&self, source_rep: Account, amount: Amount) {
        let mut guard = self.rep_amounts.lock().unwrap();
        let source_previous = self.get(&guard, &source_rep);
        let new_amount = source_previous.wrapping_add(amount);
        self.put(&mut guard, source_rep, new_amount)
    }

    pub fn representation_put(&self, account: Account, representation: Amount) {
        let mut guard = self.rep_amounts.lock().unwrap();
        self.put(&mut guard, account, representation);
    }

    pub fn representation_get(&self, account: &Account) -> Amount {
        let guard = self.rep_amounts.lock().unwrap();
        let result = self.get(&guard, account);
        result
    }

    pub fn representation_add_dual(
        &self,
        source_rep_1: Account,
        amount_1: Amount,
        source_rep_2: Account,
        amount_2: Amount,
    ) {
        if source_rep_1 != source_rep_2 {
            let mut guard = self.rep_amounts.lock().unwrap();
            let source_previous_1 = self.get(&guard, &source_rep_1);
            self.put(
                &mut guard,
                source_rep_1,
                source_previous_1.wrapping_add(amount_1),
            );
            let source_previous_2 = self.get(&guard, &source_rep_2);
            self.put(
                &mut guard,
                source_rep_2,
                source_previous_2.wrapping_add(amount_2),
            );
        } else {
            self.representation_add(source_rep_1, amount_1.wrapping_add(amount_2));
        }
    }

    pub fn item_size() -> usize {
        size_of::<(Account, Amount)>()
    }

    pub fn count(&self) -> usize {
        self.rep_amounts.lock().unwrap().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn representation_changes() {
        let account = Account::from(1);
        let rep_weights = RepWeights::new();
        assert_eq!(rep_weights.representation_get(&account), Amount::zero());

        rep_weights.representation_put(account, Amount::from(1));
        assert_eq!(rep_weights.representation_get(&account), Amount::from(1));

        rep_weights.representation_put(account, Amount::from(2));
        assert_eq!(rep_weights.representation_get(&account), Amount::from(2));
    }
}
