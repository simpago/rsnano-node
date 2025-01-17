use super::{OnlineReps, OnlineWeightSampler};
use crate::utils::{CancellationToken, Runnable};
use std::sync::{Arc, Mutex};
use tracing::info;

pub struct OnlineWeightCalculation {
    sampler: OnlineWeightSampler,
    online_reps: Arc<Mutex<OnlineReps>>,
}

impl OnlineWeightCalculation {
    pub fn new(sampler: OnlineWeightSampler, online_reps: Arc<Mutex<OnlineReps>>) -> Self {
        Self {
            sampler,
            online_reps,
        }
    }
}

impl Runnable for OnlineWeightCalculation {
    fn run(&mut self, _cancel_token: &CancellationToken) {
        let online_weight = self.online_reps.lock().unwrap().online_weight();
        self.sampler.add_sample(online_weight);
        let trend = self.sampler.calculate_trend();
        info!("Trended weight updated: {}", trend.format_balance(0));
        self.online_reps.lock().unwrap().set_trended(trend);
    }
}
