use rsnano_core::utils::{nano_seconds_since_epoch, system_time_as_nanoseconds};
use rsnano_core::{Amount, Networks};
use rsnano_ledger::Ledger;
use rsnano_store_lmdb::LmdbWriteTransaction;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

pub struct OnlineWeightSampler {
    ledger: Arc<Ledger>,

    /// The maximum time to keep online weight samples
    cutoff: Duration,
}

impl OnlineWeightSampler {
    pub fn new(ledger: Arc<Ledger>, network: Networks) -> Self {
        Self {
            ledger,
            cutoff: Self::cutoff_for(network),
        }
    }

    fn cutoff_for(network: Networks) -> Duration {
        match network {
            Networks::NanoLiveNetwork | Networks::NanoTestNetwork => {
                // Two weeks
                Duration::from_secs(60 * 60 * 24 * 7 * 2)
            }
            _ => {
                // One day
                Duration::from_secs(60 * 60 * 24)
            }
        }
    }

    pub fn calculate_trend(&self) -> Amount {
        self.medium_weight(self.load_samples())
    }

    fn load_samples(&self) -> Vec<Amount> {
        let txn = self.ledger.read_txn();
        self.ledger
            .store
            .online_weight
            .iter(&txn)
            .map(|(_, amount)| amount)
            .collect()
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

    /// Called periodically to sample online weight
    pub fn add_sample(&self, current_online_weight: Amount) {
        let mut txn = self.ledger.rw_txn();
        self.sanitize_samples(&mut txn);
        self.insert_new_sample(&mut txn, current_online_weight);
    }

    fn sanitize_samples(&self, tx: &mut LmdbWriteTransaction) {
        let now = SystemTime::now();
        let to_delete = self.samples_to_delete(tx, now);

        for timestamp in to_delete {
            self.ledger.store.online_weight.del(tx, timestamp);
        }
    }

    fn samples_to_delete(&self, tx: &LmdbWriteTransaction, now: SystemTime) -> Vec<u64> {
        let mut to_delete = Vec::new();
        to_delete.extend(self.old_samples(tx, now));
        to_delete.extend(self.future_samples(tx, now));
        to_delete
    }

    fn old_samples<'tx>(
        &self,
        tx: &'tx LmdbWriteTransaction,
        now: SystemTime,
    ) -> impl Iterator<Item = u64> + use<'tx> {
        let timestamp_cutoff = system_time_as_nanoseconds(now - self.cutoff);

        self.ledger
            .store
            .online_weight
            .iter(tx)
            .map(|(ts, _)| ts)
            .take_while(move |ts| *ts < timestamp_cutoff)
    }

    fn future_samples<'tx>(
        &self,
        tx: &'tx LmdbWriteTransaction,
        now: SystemTime,
    ) -> impl Iterator<Item = u64> + use<'tx> {
        let timestamp_now = system_time_as_nanoseconds(now);

        self.ledger
            .store
            .online_weight
            .iter_rev(tx)
            .map(|(ts, _)| ts)
            .take_while(move |ts| *ts > timestamp_now)
    }

    fn insert_new_sample(&self, txn: &mut LmdbWriteTransaction, current_online_weight: Amount) {
        self.ledger.store.online_weight.put(
            txn,
            nano_seconds_since_epoch(),
            &current_online_weight,
        );
    }
}
