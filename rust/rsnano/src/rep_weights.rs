use std::collections::HashMap;
use std::sync::Mutex;
use crate::{Account, Amount};

pub struct RepWeights {
    rep_amounts: Mutex<HashMap<Account, u128>>,
}

impl RepWeights {
    pub fn new() -> Self {
        RepWeights {
            rep_amounts: Mutex::new(HashMap::new())
        }
    }

    pub fn get(&self, account_a: &Account) -> u128 {
        let mut guard = self.rep_amounts.lock().unwrap();
        let mut amount = 0;
        match guard.get(account_a) {
            Some(a) => amount = *a,
            None => (),
        }
        amount
    }

    pub fn put(&mut self, account_a: Account, representation_a: u128) {
        let mut guard = self.rep_amounts.lock().unwrap();
        guard.insert(account_a, representation_a);
    }

    pub fn get_rep_amounts(&self) -> HashMap<Account, u128> {
        self.rep_amounts.lock().unwrap().clone()
    }

    pub fn copy_from(&mut self, other_a: RepWeights) {
        let mut guard_this = self.rep_amounts.lock().unwrap();
        let mut guard_other = other_a.rep_amounts.lock().unwrap();
        for (account, amount) in guard_other.iter() {
            match guard_this.clone().get(&account) {
                Some(a) => guard_this.insert(*account, a + amount),
                None => guard_this.insert(*account, *amount),
            };
        }
    }

    pub fn representation_add(&mut self, source_rep_a: Account, amount_a: u128) {
        let mut guard = self.rep_amounts.lock().unwrap();
        let mut source_previous: u128 = 0;
        match guard.get(&source_rep_a) {
            Some(amount_previous) => source_previous = *amount_previous,
            None => (),
        }
        guard.insert(source_rep_a, source_previous + amount_a);
    }

    pub fn representation_put(&mut self, account_a: Account, representation_a: u128) {
        self.put(account_a, representation_a);
    }

    pub fn representation_get(&mut self, account_a: &Account) -> u128 {
        self.get(account_a)
    }

    pub fn representation_add_dual(&mut self, source_rep_1: Account, amount_1: u128, source_rep_2: Account, amount_2: u128) {
        if source_rep_1 != source_rep_2 {
            self.representation_add(source_rep_1, amount_1);
            self.representation_add(source_rep_2, amount_2);
        }
        else {
            self.representation_add(source_rep_1, amount_1 + amount_2);
        }
    }
}