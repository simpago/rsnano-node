use std::{
    ffi::CStr,
    ops::Deref,
    sync::{Arc, Mutex},
};

use super::generate_cache::GenerateCacheHandle;
use crate::{ffi::StringDto, ConfirmationHeightMode, NodeFlags};
use num_traits::FromPrimitive;

pub struct NodeFlagsHandle(Arc<Mutex<NodeFlags>>);

impl NodeFlagsHandle {
    pub fn new(flags: Arc<Mutex<NodeFlags>>) -> *mut NodeFlagsHandle {
        Box::into_raw(Box::new(NodeFlagsHandle(flags)))
    }
}

impl Deref for NodeFlagsHandle {
    type Target = Arc<Mutex<NodeFlags>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[no_mangle]
pub extern "C" fn rsn_node_flags_create() -> *mut NodeFlagsHandle {
    NodeFlagsHandle::new(Arc::new(Mutex::new(NodeFlags::new())))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_node_flags_destroy(handle: *mut NodeFlagsHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_node_flags_clone(
    handle: *mut NodeFlagsHandle,
) -> *mut NodeFlagsHandle {
    NodeFlagsHandle::new(Arc::new(Mutex::new((*handle).0.lock().unwrap().clone())))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_node_flags_config_overrides(
    handle: *mut NodeFlagsHandle,
    result: *mut StringDto,
    size: usize,
) -> usize {
    let lock = (*handle).0.lock().unwrap();
    let result = std::slice::from_raw_parts_mut(result, size);
    for (i, s) in lock.config_overrides.iter().enumerate() {
        result[i] = StringDto::from(s);
    }
    lock.config_overrides.len()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_node_flags_config_set_overrides(
    handle: *mut NodeFlagsHandle,
    overrides: *const *const i8,
    size: usize,
) {
    let slice = std::slice::from_raw_parts(overrides, size);
    let overrides = slice
        .iter()
        .map(|&i| CStr::from_ptr(i).to_str().unwrap().to_string())
        .collect();
    let mut lock = (*handle).0.lock().unwrap();
    lock.config_overrides = overrides;
}

#[no_mangle]
pub unsafe extern "C" fn rsn_node_flags_rpc_config_overrides(
    handle: *mut NodeFlagsHandle,
    result: *mut StringDto,
    size: usize,
) -> usize {
    let lock = (*handle).0.lock().unwrap();
    let result = std::slice::from_raw_parts_mut(result, size);
    for (i, s) in lock.rpc_config_overrides.iter().enumerate() {
        result[i] = StringDto::from(s);
    }
    lock.rpc_config_overrides.len()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_node_flags_rpc_config_set_overrides(
    handle: *mut NodeFlagsHandle,
    overrides: *const *const i8,
    size: usize,
) {
    let slice = std::slice::from_raw_parts(overrides, size);
    let overrides = slice
        .iter()
        .map(|&i| CStr::from_ptr(i).to_str().unwrap().to_string())
        .collect();
    let mut lock = (*handle).0.lock().unwrap();
    lock.rpc_config_overrides = overrides;
}

#[no_mangle]
pub unsafe extern "C" fn rsn_node_flags_generate_cache(
    handle: *mut NodeFlagsHandle,
) -> *mut GenerateCacheHandle {
    GenerateCacheHandle::new((*handle).0.lock().unwrap().generate_cache.clone())
}

#[no_mangle]
pub unsafe extern "C" fn rsn_node_flags_generate_set_cache(
    handle: *mut NodeFlagsHandle,
    cache: *mut GenerateCacheHandle,
) {
    (*handle).0.lock().unwrap().generate_cache = (*cache).clone()
}

#[repr(C)]
pub struct NodeFlagsDto {
    pub disable_add_initial_peers: bool,
    pub disable_backup: bool,
    pub disable_lazy_bootstrap: bool,
    pub disable_legacy_bootstrap: bool,
    pub disable_wallet_bootstrap: bool,
    pub disable_bootstrap_listener: bool,
    pub disable_bootstrap_bulk_pull_server: bool,
    pub disable_bootstrap_bulk_push_client: bool,
    pub disable_ongoing_bootstrap: bool,
    pub disable_rep_crawler: bool,
    pub disable_request_loop: bool,
    pub disable_tcp_realtime: bool,
    pub disable_udp: bool,
    pub disable_unchecked_cleanup: bool,
    pub disable_unchecked_drop: bool,
    pub disable_providing_telemetry_metrics: bool,
    pub disable_ongoing_telemetry_requests: bool,
    pub disable_initial_telemetry_requests: bool,
    pub disable_block_processor_unchecked_deletion: bool,
    pub disable_block_processor_republishing: bool,
    pub allow_bootstrap_peers_duplicates: bool,
    pub disable_max_peers_per_ip: bool,
    pub disable_max_peers_per_subnetwork: bool,
    pub force_use_write_database_queue: bool,
    pub disable_search_pending: bool,
    pub enable_pruning: bool,
    pub fast_bootstrap: bool,
    pub read_only: bool,
    pub disable_connection_cleanup: bool,
    pub confirmation_height_processor_mode: u8,
    pub inactive_node: bool,
    pub block_processor_batch_size: usize,
    pub block_processor_full_size: usize,
    pub block_processor_verification_size: usize,
    pub inactive_votes_cache_size: usize,
    pub vote_processor_capacity: usize,
    pub bootstrap_interval: usize,
}

#[no_mangle]
pub unsafe extern "C" fn rsn_node_flags_get(
    handle: *mut NodeFlagsHandle,
    result: *mut NodeFlagsDto,
) {
    let lock = (*handle).0.lock().unwrap();
    let result = &mut *result;
    result.disable_add_initial_peers = lock.disable_add_initial_peers;
    result.disable_backup = lock.disable_backup;
    result.disable_lazy_bootstrap = lock.disable_lazy_bootstrap;
    result.disable_legacy_bootstrap = lock.disable_legacy_bootstrap;
    result.disable_wallet_bootstrap = lock.disable_wallet_bootstrap;
    result.disable_bootstrap_listener = lock.disable_bootstrap_listener;
    result.disable_bootstrap_bulk_pull_server = lock.disable_bootstrap_bulk_pull_server;
    result.disable_bootstrap_bulk_push_client = lock.disable_bootstrap_bulk_push_client;
    result.disable_ongoing_bootstrap = lock.disable_ongoing_bootstrap;
    result.disable_rep_crawler = lock.disable_rep_crawler;
    result.disable_request_loop = lock.disable_request_loop;
    result.disable_tcp_realtime = lock.disable_tcp_realtime;
    result.disable_udp = lock.disable_udp;
    result.disable_unchecked_cleanup = lock.disable_unchecked_cleanup;
    result.disable_unchecked_drop = lock.disable_unchecked_drop;
    result.disable_providing_telemetry_metrics = lock.disable_providing_telemetry_metrics;
    result.disable_ongoing_telemetry_requests = lock.disable_ongoing_telemetry_requests;
    result.disable_initial_telemetry_requests = lock.disable_initial_telemetry_requests;
    result.disable_block_processor_unchecked_deletion =
        lock.disable_block_processor_unchecked_deletion;
    result.disable_block_processor_republishing = lock.disable_block_processor_republishing;
    result.allow_bootstrap_peers_duplicates = lock.allow_bootstrap_peers_duplicates;
    result.disable_max_peers_per_ip = lock.disable_max_peers_per_ip;
    result.disable_max_peers_per_subnetwork = lock.disable_max_peers_per_subnetwork;
    result.force_use_write_database_queue = lock.force_use_write_database_queue;
    result.disable_search_pending = lock.disable_search_pending;
    result.enable_pruning = lock.enable_pruning;
    result.fast_bootstrap = lock.fast_bootstrap;
    result.read_only = lock.read_only;
    result.disable_connection_cleanup = lock.disable_connection_cleanup;
    result.confirmation_height_processor_mode = lock.confirmation_height_processor_mode as u8;
    result.inactive_node = lock.inactive_node;
    result.block_processor_batch_size = lock.block_processor_batch_size;
    result.block_processor_full_size = lock.block_processor_full_size;
    result.block_processor_verification_size = lock.block_processor_verification_size;
    result.inactive_votes_cache_size = lock.inactive_votes_cache_size;
    result.vote_processor_capacity = lock.vote_processor_capacity;
    result.bootstrap_interval = lock.bootstrap_interval;
}

#[no_mangle]
pub unsafe extern "C" fn rsn_node_flags_set(
    handle: *mut NodeFlagsHandle,
    flags: *const NodeFlagsDto,
) {
    let flags = &*flags;
    let mut lock = (*handle).0.lock().unwrap();
    lock.disable_add_initial_peers = flags.disable_add_initial_peers;
    lock.disable_backup = flags.disable_backup;
    lock.disable_lazy_bootstrap = flags.disable_lazy_bootstrap;
    lock.disable_legacy_bootstrap = flags.disable_legacy_bootstrap;
    lock.disable_wallet_bootstrap = flags.disable_wallet_bootstrap;
    lock.disable_bootstrap_listener = flags.disable_bootstrap_listener;
    lock.disable_bootstrap_bulk_pull_server = flags.disable_bootstrap_bulk_pull_server;
    lock.disable_bootstrap_bulk_push_client = flags.disable_bootstrap_bulk_push_client;
    lock.disable_ongoing_bootstrap = flags.disable_ongoing_bootstrap;
    lock.disable_rep_crawler = flags.disable_rep_crawler;
    lock.disable_request_loop = flags.disable_request_loop;
    lock.disable_tcp_realtime = flags.disable_tcp_realtime;
    lock.disable_udp = flags.disable_udp;
    lock.disable_unchecked_cleanup = flags.disable_unchecked_cleanup;
    lock.disable_unchecked_drop = flags.disable_unchecked_drop;
    lock.disable_providing_telemetry_metrics = flags.disable_providing_telemetry_metrics;
    lock.disable_ongoing_telemetry_requests = flags.disable_ongoing_telemetry_requests;
    lock.disable_initial_telemetry_requests = flags.disable_initial_telemetry_requests;
    lock.disable_block_processor_unchecked_deletion =
        flags.disable_block_processor_unchecked_deletion;
    lock.disable_block_processor_republishing = flags.disable_block_processor_republishing;
    lock.allow_bootstrap_peers_duplicates = flags.allow_bootstrap_peers_duplicates;
    lock.disable_max_peers_per_ip = flags.disable_max_peers_per_ip;
    lock.disable_max_peers_per_subnetwork = flags.disable_max_peers_per_subnetwork;
    lock.force_use_write_database_queue = flags.force_use_write_database_queue;
    lock.disable_search_pending = flags.disable_search_pending;
    lock.enable_pruning = flags.enable_pruning;
    lock.fast_bootstrap = flags.fast_bootstrap;
    lock.read_only = flags.read_only;
    lock.disable_connection_cleanup = flags.disable_connection_cleanup;
    lock.confirmation_height_processor_mode =
        ConfirmationHeightMode::from_u8(flags.confirmation_height_processor_mode).unwrap();
    lock.inactive_node = flags.inactive_node;
    lock.block_processor_batch_size = flags.block_processor_batch_size;
    lock.block_processor_full_size = flags.block_processor_full_size;
    lock.block_processor_verification_size = flags.block_processor_verification_size;
    lock.inactive_votes_cache_size = flags.inactive_votes_cache_size;
    lock.vote_processor_capacity = flags.vote_processor_capacity;
    lock.bootstrap_interval = flags.bootstrap_interval;
}
