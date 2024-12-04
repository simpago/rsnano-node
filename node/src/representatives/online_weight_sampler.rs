use rsnano_core::utils::nano_seconds_since_epoch;
use rsnano_core::Amount;
use rsnano_ledger::Ledger;
use rsnano_store_lmdb::LmdbWriteTransaction;
use std::sync::Arc;

pub struct OnlineWeightSampler {
    ledger: Arc<Ledger>,
    max_samples: usize,
}

impl OnlineWeightSampler {
    pub fn new(ledger: Arc<Ledger>, max_samples: usize) -> Self {
        Self {
            ledger,
            max_samples,
        }
    }

    pub fn calculate_trend(&self) -> Amount {
        self.medium_weight(self.load_samples())
    }

    fn load_samples(&self) -> Vec<Amount> {
        let txn = self.ledger.read_txn();
        let mut items = Vec::with_capacity(self.max_samples as usize + 1);
        let mut it = self.ledger.store.online_weight.begin(&txn);
        while !it.is_end() {
            items.push(*it.current().unwrap().1);
            it.next();
        }
        items
    }

    fn medium_weight(&self, mut items: Vec<Amount>) -> Amount {
        if items.is_empty() {
            Amount::zero()
        } else {
            let median_idx = items.len() / 2;
            items.sort();
            items[median_idx]
        }
    }

    /** Called periodically to sample online weight */
    pub fn sample(&self, current_online_weight: Amount) {
        let mut txn = self.ledger.rw_txn();
        self.delete_old_samples(&mut txn);
        self.insert_new_sample(&mut txn, current_online_weight);
    }

    fn delete_old_samples(&self, txn: &mut LmdbWriteTransaction) {
        let weight_store = &self.ledger.store.online_weight;

        while weight_store.count(txn) >= self.max_samples as u64 {
            let (&oldest, _) = weight_store.begin(txn).current().unwrap();
            weight_store.del(txn, oldest);
        }
    }

    fn insert_new_sample(&self, txn: &mut LmdbWriteTransaction, current_online_weight: Amount) {
        self.ledger.store.online_weight.put(
            txn,
            nano_seconds_since_epoch(),
            &current_online_weight,
        );
    }
}
