use super::{OnlineReps, OnlineWeightSampler};
use crate::utils::{CancellationToken, Runnable};
use std::sync::{Arc, Mutex};
use tracing::info;

pub struct OnlineWeightCalculation {
    sampler: OnlineWeightSampler,
    online_reps: Arc<Mutex<OnlineReps>>,
    first_run: bool,
}

impl OnlineWeightCalculation {
    pub fn new(sampler: OnlineWeightSampler, online_reps: Arc<Mutex<OnlineReps>>) -> Self {
        Self {
            sampler,
            online_reps,
            first_run: true,
        }
    }
}

impl Runnable for OnlineWeightCalculation {
    fn run(&mut self, _: &CancellationToken) {
        if self.first_run {
            // Don't sample online weight on first run, because it is always 0
            self.first_run = false;
            self.sampler.sanitize();
        } else {
            let online_weight = self.online_reps.lock().unwrap().online_weight();
            self.sampler.add_sample(online_weight);
        }
        let result = self.sampler.calculate_trend();
        info!(
            "Trended weight updated: {}, samples: {}",
            result.trended.format_balance(0),
            result.sample_count
        );
        self.online_reps.lock().unwrap().set_trended(result.trended);
    }
}
