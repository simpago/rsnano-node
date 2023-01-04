use crate::config::NodeConfig;
use crate::stats::DetailType::Send;
use crate::voting::VoteSpacing;
use blake2::digest::typenum::private::Trim;
use primitive_types::U256;
use rsnano_core::{u256_struct, Account, Amount, BlockHash, Root};
use rsnano_ledger::Ledger;
use rsnano_store_lmdb::get;
use rsnano_store_traits::{ReadTransaction, Transaction, WriteTransaction};
use std::cmp::max;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::mem;
use std::mem::size_of;
use std::ops::Deref;
use std::process::id;
use std::sync::{Arc, Mutex};
use std::thread::current;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const ONLINE_WEIGHT_QUORUM: u8 = 67;

pub struct OnlineReps {
    pub ledger: Arc<Ledger>,
    pub node_config: Arc<NodeConfig>,
    reps: EntryContainer,
    pub trended_m: Arc<Mutex<Amount>>,
    pub online_m: Arc<Mutex<Amount>>,
    pub minimum: Arc<Mutex<Amount>>,
}

impl OnlineReps {
    pub fn new(ledger: Arc<Ledger>, node_config: Arc<NodeConfig>) -> Self {
        let transaction = ledger.store.tx_begin_read().unwrap();

        let mut online_reps = Self {
            ledger,
            node_config,
            reps: EntryContainer::new(),
            trended_m: Arc::new(Mutex::new(Amount::zero())),
            online_m: Arc::new(Mutex::new(Amount::zero())),
            minimum: Arc::new(Mutex::new(Amount::zero())),
        };

        let mut mutex = online_reps.trended_m.lock().unwrap();
        *mutex = online_reps.calculate_trend(transaction.txn());
        std::mem::drop(mutex);
        online_reps
    }

    pub fn calculate_online(&self) -> Amount {
        let mut current = 0;
        for (_, e) in self.reps.entries.lock().unwrap().iter() {
            current += self.ledger.weight(&e.account).number();
        }
        Amount::new(current)
    }

    pub fn calculate_trend(&self, transaction_a: &dyn Transaction) -> Amount {
        let mut items = Vec::new();
        items.push(self.node_config.online_weight_minimum);
        let mut it = self.ledger.store.online_weight().begin(transaction_a);
        while !it.is_end() {
            items.push(*it.current().unwrap().1);
            it.next();
        }
        let median_idx = items.len() / 2;
        items.sort();
        let result = items[median_idx];
        return result;
    }

    pub fn observe(&mut self, rep_a: Account) {
        if self.ledger.weight(&rep_a).number() > 0 {
            let mut new_insert = true;
            let mut by_account_mutex = self.reps.by_account.lock().unwrap();
            let mut by_time_mutex = self.reps.by_time.lock().unwrap();
            let mut entries_mutex = self.reps.entries.lock().unwrap();
            if let Some(id) = by_account_mutex.get(&rep_a) {
                let old_time = entries_mutex.get(id).unwrap().time;
                let mut ids = by_time_mutex.get(&old_time).unwrap().clone();
                let index = ids.iter().position(|x| x == id).unwrap();
                ids.remove(index);
                by_time_mutex.insert(old_time, ids.to_owned());
                entries_mutex.remove(id).unwrap();
                new_insert = false;
            }
            by_account_mutex.remove(&rep_a);
            mem::drop(by_account_mutex);
            mem::drop(by_time_mutex);
            mem::drop(entries_mutex);
            let time = Instant::now();
            let entry = Entry {
                account: rep_a,
                time,
            };
            self.reps.insert(entry);
            println!("weight: {}", self.node_config.weight_period); // should't it be 300?
            let trimmed = self
                .reps
                .trim(Duration::from_secs(self.node_config.weight_period));
            if new_insert || trimmed {
                let mut mutex = self.online_m.lock().unwrap();
                *mutex = self.calculate_online();
            }
        }
    }

    pub fn sample(&mut self) {
        let mut transaction = self.ledger.store.tx_begin_write().unwrap();
        while self.ledger.store.online_weight().count(transaction.txn())
            >= self.node_config.max_weight_samples
        {
            let oldest = self.ledger.store.online_weight().begin(transaction.txn());
            debug_assert!(
                oldest.as_ref().current().unwrap()
                    != self
                        .ledger
                        .store
                        .online_weight()
                        .rbegin(transaction.txn())
                        .current()
                        .unwrap()
            );
            self.ledger
                .store
                .online_weight()
                .del(transaction.as_mut(), *oldest.current().unwrap().0);
        }
        let start = SystemTime::now();
        let since_the_epoch = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        self.ledger.store.online_weight().put(
            transaction.as_mut(),
            since_the_epoch.as_secs(),
            &self.online_m.lock().unwrap(),
        );
        let mut mutex = self.trended_m.lock().unwrap();
        *mutex = self.calculate_trend(transaction.txn());
    }

    pub fn trended(&self) -> Amount {
        self.trended_m.lock().unwrap().clone()
    }

    pub fn online(&self) -> Amount {
        let online = *self.online_m.lock().unwrap();
        online
    }

    pub fn delta(&self) -> Amount {
        let weight = max(
            self.online_m.lock().unwrap().clone(),
            self.trended_m.lock().unwrap().deref().clone(),
        );
        let weight = max(weight, self.node_config.online_weight_minimum);
        let amount =
            U256::from(weight.number()) * U256::from(ONLINE_WEIGHT_QUORUM) / U256::from(100);
        return Amount::new(amount.as_u128());
    }

    pub fn list(&self) -> Vec<Account> {
        self.reps
            .by_account
            .lock()
            .unwrap()
            .iter()
            .map(|(a, b)| *a)
            .collect()
    }

    pub fn clear(&mut self) {
        self.reps = EntryContainer::new();
        let mut mutex2 = self.online_m.lock().unwrap();
        *mutex2 = Amount::zero();
    }

    pub fn count(&self) -> usize {
        self.reps.len()
    }

    pub fn item_size() -> usize {
        size_of::<(usize, Entry)>()
    }
}

#[derive(Default)]
struct EntryContainer {
    entries: Arc<Mutex<HashMap<usize, Entry>>>,
    by_account: Arc<Mutex<HashMap<Account, usize>>>,
    by_time: Arc<Mutex<BTreeMap<Instant, Vec<usize>>>>,
    next_id: Arc<Mutex<usize>>,
}

impl EntryContainer {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn clear(&mut self) {
        self.entries = Arc::new(Mutex::new(HashMap::new()));
        self.by_account = Arc::new(Mutex::new(HashMap::new()));
        self.by_time = Arc::new(Mutex::new(BTreeMap::new()));
        self.next_id = Arc::new(Mutex::new(0));
    }

    pub fn insert(&mut self, entry: Entry) {
        let id = self.create_id();

        self.by_account.lock().unwrap().insert(entry.account, id);

        let mut binding = self.by_time.lock().unwrap();
        let mut by_time = binding.entry(entry.time).or_default();
        by_time.push(id);

        self.entries.lock().unwrap().insert(id, entry);
    }

    fn create_id(&mut self) -> usize {
        let mut id = self.next_id.lock().unwrap();
        *id += 1;
        *id
    }

    pub fn by_account(&self, account: &Account) -> Option<usize> {
        match self.by_account.lock().unwrap().get(account) {
            Some(id) => Some(*id),
            None => None,
        }
    }

    fn trim(&mut self, upper_bound: Duration) -> bool {
        let mut trimmed = false;
        let mut instants_to_remove = Vec::new();
        for (&instant, ids) in self.by_time.lock().unwrap().iter() {
            if instant.elapsed() < upper_bound {
                break;
            }

            instants_to_remove.push(instant);

            for id in ids {
                let entry = self.entries.lock().unwrap().remove(id).unwrap();
                self.by_account
                    .lock()
                    .unwrap()
                    .remove(&entry.account)
                    .unwrap();
            }
            trimmed = true;
        }

        for instant in instants_to_remove {
            self.by_time.lock().unwrap().remove(&instant);
        }

        trimmed
    }

    fn len(&self) -> usize {
        self.entries.lock().unwrap().len()
    }
}

struct Entry {
    account: Account,
    time: Instant,
}
