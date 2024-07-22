use crate::U256ArrayDto;
use rsnano_core::Amount;
use rsnano_node::representatives::{OnlineReps, ONLINE_WEIGHT_QUORUM};
use rsnano_node::OnlineWeightSampler;
use std::ops::Deref;
use std::sync::{Arc, Mutex};

pub struct OnlineRepsHandle {
    pub online_reps: Arc<Mutex<OnlineReps>>,
    pub sampler: Arc<OnlineWeightSampler>,
}

impl Deref for OnlineRepsHandle {
    type Target = Arc<Mutex<OnlineReps>>;

    fn deref(&self) -> &Self::Target {
        &self.online_reps
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_online_reps_destroy(handle: *mut OnlineRepsHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_online_reps_set_online(
    handle: *mut OnlineRepsHandle,
    online: *const u8,
) {
    let amount = Amount::from_ptr(online);
    (*handle).online_reps.lock().unwrap().set_online(amount);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_online_reps_online_weight_quorum() -> u8 {
    ONLINE_WEIGHT_QUORUM
}

#[no_mangle]
pub unsafe extern "C" fn rsn_online_reps_list(
    handle: *mut OnlineRepsHandle,
    result: *mut U256ArrayDto,
) {
    let accounts = (*handle).online_reps.lock().unwrap().list();
    let data = accounts.iter().map(|a| *a.as_bytes()).collect();
    (*result).initialize(data);
}
