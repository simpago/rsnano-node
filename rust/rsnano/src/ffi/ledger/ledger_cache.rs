use std::{
    ops::Deref,
    sync::{atomic::Ordering, Arc},
};

use crate::ledger::LedgerCache;

use super::RepWeightsHandle;

pub struct LedgerCacheHandle(Arc<LedgerCache>);

impl LedgerCacheHandle {
    pub fn new(cache: Arc<LedgerCache>) -> *mut Self {
        Box::into_raw(Box::new(LedgerCacheHandle(cache)))
    }
}

impl Deref for LedgerCacheHandle {
    type Target = Arc<LedgerCache>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub extern "C" fn rsn_ledger_cache_create() -> *mut LedgerCacheHandle {
    LedgerCacheHandle::new(Arc::new(LedgerCache::new()))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_ledger_cache_destroy(handle: *mut LedgerCacheHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_ledger_cache_weights(
    handle: *mut LedgerCacheHandle,
) -> *mut RepWeightsHandle {
    RepWeightsHandle::new((*handle).0.rep_weights.clone())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_ledger_cache_cemented_count(handle: *mut LedgerCacheHandle) -> u64 {
    (*handle).0.cemented_count.load(Ordering::SeqCst)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_ledger_cache_block_count(handle: *mut LedgerCacheHandle) -> u64 {
    (*handle).0.block_count.load(Ordering::SeqCst)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_ledger_cache_pruned_count(handle: *mut LedgerCacheHandle) -> u64 {
    (*handle).0.pruned_count.load(Ordering::SeqCst)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_ledger_cache_account_count(handle: *mut LedgerCacheHandle) -> u64 {
    (*handle).0.account_count.load(Ordering::SeqCst)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_ledger_cache_final_votes_confirmation_canary(
    handle: *mut LedgerCacheHandle,
) -> bool {
    (*handle)
        .0
        .final_votes_confirmation_canary
        .load(Ordering::SeqCst)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_ledger_cache_add_cemented(handle: *mut LedgerCacheHandle, count: u64) {
    (*handle)
        .0
        .cemented_count
        .fetch_add(count, Ordering::SeqCst);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_ledger_cache_add_blocks(handle: *mut LedgerCacheHandle, count: u64) {
    (*handle).0.block_count.fetch_add(count, Ordering::SeqCst);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_ledger_cache_add_pruned(handle: *mut LedgerCacheHandle, count: u64) {
    (*handle).0.pruned_count.fetch_add(count, Ordering::SeqCst);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_ledger_cache_add_accounts(handle: *mut LedgerCacheHandle, count: u64) {
    (*handle).0.account_count.fetch_add(count, Ordering::SeqCst);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_ledger_cache_remove_accounts(
    handle: *mut LedgerCacheHandle,
    count: u64,
) {
    (*handle).0.account_count.fetch_sub(count, Ordering::SeqCst);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_ledger_cache_remove_blocks(
    handle: *mut LedgerCacheHandle,
    count: u64,
) {
    (*handle).0.block_count.fetch_sub(count, Ordering::SeqCst);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_ledger_cache_set_final_votes_confirmation_canary(
    handle: *mut LedgerCacheHandle,
    value: bool,
) {
    (*handle)
        .0
        .final_votes_confirmation_canary
        .store(value, Ordering::SeqCst);
}
