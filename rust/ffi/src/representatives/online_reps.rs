use crate::ledger::datastore::LedgerHandle;
use crate::{copy_amount_bytes, U256ArrayDto};
use rsnano_core::{Account, Amount};
use rsnano_node::representatives::{OnlineReps, ONLINE_WEIGHT_QUORUM};
use rsnano_node::OnlineWeightSampler;
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct OnlineRepsHandle {
    pub online_reps: Arc<Mutex<OnlineReps>>,
    pub sampler: OnlineWeightSampler,
}

impl Deref for OnlineRepsHandle {
    type Target = Arc<Mutex<OnlineReps>>;

    fn deref(&self) -> &Self::Target {
        &self.online_reps
    }
}

#[no_mangle]
pub unsafe extern "C" fn rsn_online_reps_create(
    ledger_handle: *mut LedgerHandle,
    weight_period_s: u64,
    online_weight_minimum: *const u8,
    max_samples: u64,
) -> *mut OnlineRepsHandle {
    let online_weight_minimum = Amount::from_ptr(online_weight_minimum);
    let weight_period = Duration::from_secs(weight_period_s);

    let mut online_reps = OnlineReps::new((*ledger_handle).clone());
    online_reps.set_weight_period(weight_period);
    online_reps.set_online_weight_minimum(online_weight_minimum);

    let mut sampler = OnlineWeightSampler::new((*ledger_handle).clone());
    sampler.set_online_weight_minimum(online_weight_minimum);
    sampler.set_max_samples(max_samples);

    online_reps.set_trended(sampler.calculate_trend());

    let handle = OnlineRepsHandle {
        online_reps: Arc::new(Mutex::new(online_reps)),
        sampler,
    };

    Box::into_raw(Box::new(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_online_reps_destroy(handle: *mut OnlineRepsHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_online_reps_observe(
    handle: *mut OnlineRepsHandle,
    rep_account: *const u8,
) {
    let rep_account = Account::from_ptr(rep_account);
    (*handle).online_reps.lock().unwrap().observe(rep_account)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_online_reps_sample(handle: *mut OnlineRepsHandle) {
    let online = {
        let lock = (*handle).online_reps.lock().unwrap();
        lock.online()
    };

    (*handle).sampler.sample(online);
    let trend = (*handle).sampler.calculate_trend();

    let mut lock = (*handle).online_reps.lock().unwrap();
    lock.set_trended(trend);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_online_reps_trended(handle: *mut OnlineRepsHandle, result: *mut u8) {
    let amount = (*handle).online_reps.lock().unwrap().trended();
    copy_amount_bytes(amount, result);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_online_reps_online(handle: *mut OnlineRepsHandle, result: *mut u8) {
    let amount = (*handle).online_reps.lock().unwrap().online();
    copy_amount_bytes(amount, result);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_online_reps_delta(handle: *mut OnlineRepsHandle, result: *mut u8) {
    let amount = (*handle).online_reps.lock().unwrap().delta();
    copy_amount_bytes(amount, result);
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

#[no_mangle]
pub unsafe extern "C" fn rsn_online_reps_clear(handle: *mut OnlineRepsHandle) {
    (*handle).online_reps.lock().unwrap().clear();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_online_reps_item_count(handle: *const OnlineRepsHandle) -> usize {
    (*handle).online_reps.lock().unwrap().count()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_online_reps_item_size() -> usize {
    OnlineReps::item_size()
}
