use crate::{
    copy_amount_bytes,
    ledger::datastore::LedgerHandle,
    representatives::OnlineRepsHandle,
    utils::{ContainerInfoComponentHandle, ContextWrapper},
    voting::VoteHandle,
    NodeConfigDto, NodeFlagsHandle, VoidPointerCallback,
};
use core::ffi::c_void;
use rsnano_core::{Account, BlockHash};
use rsnano_node::{config::NodeConfig, GapCache};
use std::{
    ffi::{c_char, CStr},
    sync::Arc,
};

pub struct GapCacheHandle(GapCache);

#[no_mangle]
pub unsafe extern "C" fn rsn_gap_cache_create(
    node_config_dto: NodeConfigDto,
    online_reps_handle: *mut OnlineRepsHandle,
    ledger_handle: *mut LedgerHandle,
    node_flags_handle: *mut NodeFlagsHandle,
    start_bootstrap_callback: StartBootstrapCallback,
    start_bootstrap_callback_context: *mut c_void,
    drop_start_bootstrap_callback: VoidPointerCallback,
) -> *mut GapCacheHandle {
    let node_config = Arc::new(NodeConfig::try_from(&node_config_dto).unwrap());
    let ledger = (*ledger_handle).clone();
    let online_reps = Arc::clone(&*online_reps_handle);
    let node_flags = Arc::new((*node_flags_handle).0.lock().unwrap().to_owned());

    let start_bootstrap_callback = wrap_start_bootstrap_callback(
        start_bootstrap_callback,
        start_bootstrap_callback_context,
        drop_start_bootstrap_callback,
    );

    let gap_cache = GapCache::new(
        node_config,
        online_reps,
        ledger,
        node_flags,
        start_bootstrap_callback,
    );
    Box::into_raw(Box::new(GapCacheHandle(gap_cache)))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_gap_cache_destroy(handle: *mut GapCacheHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_gap_cache_add(
    handle: *mut GapCacheHandle,
    hash_a: *const u8,
    time_point_a: i64,
) {
    let hash = BlockHash::from_ptr(hash_a);
    (*handle).0.add(&hash, time_point_a);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_gap_cache_erase(handle: *mut GapCacheHandle, hash_a: *const u8) {
    let hash = BlockHash::from_ptr(hash_a);
    (*handle).0.erase(&hash);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_gap_cache_vote(
    handle: *mut GapCacheHandle,
    vote_handle: *mut VoteHandle,
) {
    (*handle).0.vote(&(*vote_handle).0.read().unwrap());
}

#[no_mangle]
pub unsafe extern "C" fn rsn_gap_cache_bootstrap_check(
    handle: *mut GapCacheHandle,
    size: usize,
    voters: *const u8,
    hash: *const u8,
) -> bool {
    let byte_slice = std::slice::from_raw_parts(voters, size);
    let chunk_size = size / 32;
    let chunks = byte_slice.chunks_exact(chunk_size);

    let voters: Vec<Account> = chunks
        .map(|chunk| Account::from_bytes(chunk.try_into().unwrap()))
        .collect();

    (*handle)
        .0
        .bootstrap_check(&voters, &BlockHash::from_ptr(hash))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_gap_cache_bootstrap_threshold(
    handle: *mut GapCacheHandle,
    result: *mut u8,
) {
    let threshold = (*handle).0.bootstrap_threshold();
    copy_amount_bytes(threshold, result);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_gap_cache_size(handle: *mut GapCacheHandle) -> usize {
    (*handle).0.size()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_gap_cache_block_exists(
    handle: *mut GapCacheHandle,
    hash_a: *const u8,
) -> bool {
    let hash = BlockHash::from_ptr(hash_a);
    (*handle).0.block_exists(&hash)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_gap_cache_earliest(handle: *mut GapCacheHandle) -> i64 {
    (*handle).0.earliest()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_gap_cache_block_arrival(
    handle: *mut GapCacheHandle,
    hash_a: *const u8,
) -> i64 {
    let hash = BlockHash::from_ptr(hash_a);
    (*handle).0.block_arrival(&hash)
}

pub type StartBootstrapCallback = unsafe extern "C" fn(*mut c_void, *const u8);

unsafe fn wrap_start_bootstrap_callback(
    callback: StartBootstrapCallback,
    context: *mut c_void,
    drop_context: VoidPointerCallback,
) -> Box<dyn Fn(BlockHash)> {
    let context_wrapper = ContextWrapper::new(context, drop_context);
    Box::new(move |block_hash: BlockHash| {
        callback(
            context_wrapper.get_context(),
            block_hash.as_bytes().as_ptr(),
        );
    })
}

#[no_mangle]
pub unsafe extern "C" fn rsn_gap_cache_collect_container_info(
    handle: *const GapCacheHandle,
    name: *const c_char,
) -> *mut ContainerInfoComponentHandle {
    let container_info = (*handle)
        .0
        .collect_container_info(CStr::from_ptr(name).to_str().unwrap().to_owned());
    Box::into_raw(Box::new(ContainerInfoComponentHandle(container_info)))
}
