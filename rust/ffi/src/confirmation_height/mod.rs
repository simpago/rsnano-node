use std::{
    ffi::c_void,
    ops::Deref,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex, RwLock, Weak,
    },
    time::Duration,
};

use rsnano_core::{BlockEnum, BlockHash};
use rsnano_node::{
    config::Logging,
    confirmation_height::{ConfHeightDetails, ConfirmationHeightUnbounded, ConfirmedIteratedPair},
};

use crate::{
    core::BlockHandle,
    ledger::datastore::{LedgerHandle, WriteDatabaseQueueHandle},
    utils::{LoggerHandle, LoggerMT},
    LoggingDto, StatHandle, VoidPointerCallback,
};

pub struct ConfirmationHeightUnboundedHandle(ConfirmationHeightUnbounded);

pub type ConfHeightUnboundedNotifyObserversCallback =
    unsafe extern "C" fn(*mut c_void, *const *mut BlockHandle, usize);

pub type ConfHeightUnboundedNotifyBlockAlreadyCementedCallback =
    unsafe extern "C" fn(*mut c_void, *const u8);

pub type AwaitingProcessingSizeCallback = unsafe extern "C" fn(*mut c_void) -> u64;

#[no_mangle]
pub unsafe extern "C" fn rsn_conf_height_unbounded_create(
    ledger: *const LedgerHandle,
    logger: *mut LoggerHandle,
    logging: *const LoggingDto,
    stats: *const StatHandle,
    batch_separate_pending_min_time_ms: u64,
    batch_write_size: *mut AtomicU64Handle,
    write_database_queue: *const WriteDatabaseQueueHandle,

    notify_observers: ConfHeightUnboundedNotifyObserversCallback,
    notify_observers_context: *mut c_void,
    drop_notify_observers_context: VoidPointerCallback,

    notify_block_already_cemented: ConfHeightUnboundedNotifyBlockAlreadyCementedCallback,
    notify_block_already_cemented_context: *mut c_void,
    drop_notify_block_already_cemented_context: VoidPointerCallback,

    awaiting_processing_size: AwaitingProcessingSizeCallback,
    awaiting_processing_size_context: *mut c_void,
    drop_awaiting_processing_size_context: VoidPointerCallback,
) -> *mut ConfirmationHeightUnboundedHandle {
    let notify_observers_callback = wrap_notify_observers_callback(
        notify_observers,
        notify_observers_context,
        drop_notify_observers_context,
    );

    let notify_block_already_cemented_callback = wrap_notify_block_already_cemented_callback(
        notify_block_already_cemented,
        notify_block_already_cemented_context,
        drop_notify_block_already_cemented_context,
    );

    let awaiting_processing_size_callback = wrap_awaiting_processing_size_callback(
        awaiting_processing_size,
        awaiting_processing_size_context,
        drop_awaiting_processing_size_context,
    );

    let result = Box::into_raw(Box::new(ConfirmationHeightUnboundedHandle(
        ConfirmationHeightUnbounded::new(
            Arc::clone(&(*ledger).0),
            Arc::new(LoggerMT::new(Box::from_raw(logger))),
            Logging::from(&*logging),
            Arc::clone(&(*stats).0),
            Duration::from_millis(batch_separate_pending_min_time_ms),
            Arc::clone(&(*batch_write_size).0),
            Arc::clone(&(*write_database_queue).0),
            notify_observers_callback,
            notify_block_already_cemented_callback,
            awaiting_processing_size_callback,
        ),
    )));
    result
}

struct ContextWrapper {
    context: *mut c_void,
    drop_context: VoidPointerCallback,
}

impl ContextWrapper {
    fn new(context: *mut c_void, drop_context: VoidPointerCallback) -> Self {
        Self {
            context,
            drop_context,
        }
    }

    fn get_context(&self) -> *mut c_void {
        self.context
    }
}

impl Drop for ContextWrapper {
    fn drop(&mut self) {
        unsafe {
            (self.drop_context)(self.context);
        }
    }
}

unsafe fn wrap_notify_observers_callback(
    callback: ConfHeightUnboundedNotifyObserversCallback,
    context: *mut c_void,
    drop_context: VoidPointerCallback,
) -> Box<dyn Fn(&Vec<Arc<BlockEnum>>)> {
    let context_wrapper = ContextWrapper::new(context, drop_context);

    Box::new(move |blocks| {
        let block_handles = blocks
            .iter()
            .map(|b| {
                Box::into_raw(Box::new(BlockHandle::new(Arc::new(RwLock::new(
                    b.deref().clone(),
                )))))
            })
            .collect::<Vec<_>>();

        callback(
            context_wrapper.get_context(),
            block_handles.as_ptr(),
            block_handles.len(),
        );

        for handle in block_handles {
            drop(Box::from_raw(handle))
        }
    })
}

unsafe fn wrap_notify_block_already_cemented_callback(
    callback: ConfHeightUnboundedNotifyBlockAlreadyCementedCallback,
    context: *mut c_void,
    drop_context: VoidPointerCallback,
) -> Box<dyn Fn(&BlockHash)> {
    let context_wrapper = ContextWrapper::new(context, drop_context);

    Box::new(move |block_hash| {
        callback(
            context_wrapper.get_context(),
            block_hash.as_bytes().as_ptr(),
        );
    })
}

unsafe fn wrap_awaiting_processing_size_callback(
    callback: AwaitingProcessingSizeCallback,
    context: *mut c_void,
    drop_context: VoidPointerCallback,
) -> Box<dyn Fn() -> u64> {
    let context_wrapper = ContextWrapper::new(context, drop_context);
    Box::new(move || callback(context_wrapper.get_context()))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_conf_height_unbounded_destroy(
    handle: *mut ConfirmationHeightUnboundedHandle,
) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_conf_height_unbounded_pending_empty(
    handle: *mut ConfirmationHeightUnboundedHandle,
) -> bool {
    (*handle).0.pending_empty()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_conf_height_unbounded_process(
    handle: *mut ConfirmationHeightUnboundedHandle,
    original_block: *mut BlockHandle,
) {
    (*handle)
        .0
        .process(Arc::new((*original_block).block.read().unwrap().clone()));
}

#[no_mangle]
pub extern "C" fn rsn_conf_height_details_size() -> usize {
    std::mem::size_of::<ConfHeightDetails>()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_conf_height_unbounded_conf_iterated_pairs_len(
    handle: *const ConfirmationHeightUnboundedHandle,
) -> usize {
    (*handle)
        .0
        .confirmed_iterated_pairs_size
        .load(Ordering::Relaxed)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_conf_height_unbounded_pending_writes_len(
    handle: *const ConfirmationHeightUnboundedHandle,
) -> usize {
    (*handle).0.pending_writes_size().load(Ordering::Relaxed)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_conf_iterated_pair_size() -> usize {
    std::mem::size_of::<ConfirmedIteratedPair>()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_conf_height_unbounded_implicit_receive_cemented_mapping_size(
    handle: *mut ConfirmationHeightUnboundedHandle,
) -> usize {
    (*handle)
        .0
        .implicit_receive_cemented_mapping_size
        .load(Ordering::Relaxed)
}

#[no_mangle]
pub extern "C" fn rsn_implicit_receive_cemented_mapping_value_size() -> usize {
    std::mem::size_of::<Weak<Mutex<ConfHeightDetails>>>()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_conf_height_unbounded_has_iterated_over_block(
    handle: *const ConfirmationHeightUnboundedHandle,
    hash: *const u8,
) -> bool {
    (*handle)
        .0
        .has_iterated_over_block(&BlockHash::from_ptr(hash))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_conf_height_unbounded_block_cache_size(
    handle: *const ConfirmationHeightUnboundedHandle,
) -> usize {
    (*handle).0.block_cache_size()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_conf_height_unbounded_block_cache_element_size() -> usize {
    std::mem::size_of::<Arc<BlockEnum>>()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_conf_height_unbounded_pending_writes_size_safe(
    handle: *mut ConfirmationHeightUnboundedHandle,
) -> usize {
    (*handle).0.pending_writes_size().load(Ordering::Relaxed)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_conf_height_unbounded_clear_process_vars(
    handle: *mut ConfirmationHeightUnboundedHandle,
) {
    (*handle).0.clear_process_vars()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_conf_height_unbounded_cement_blocks(
    handle: *mut ConfirmationHeightUnboundedHandle,
) {
    (*handle).0.cement_pending_blocks();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_conf_height_unbounded_stop(
    handle: *mut ConfirmationHeightUnboundedHandle,
) {
    (*handle).0.stop();
}

//
// BlockHashVec
//

pub struct BlockHashVecHandle(pub Vec<BlockHash>);

#[no_mangle]
pub extern "C" fn rsn_block_hash_vec_create() -> *mut BlockHashVecHandle {
    Box::into_raw(Box::new(BlockHashVecHandle(Vec::new())))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_block_hash_vec_clone(
    handle: *const BlockHashVecHandle,
) -> *mut BlockHashVecHandle {
    Box::into_raw(Box::new(BlockHashVecHandle((*handle).0.clone())))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_block_hash_vec_destroy(handle: *mut BlockHashVecHandle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_block_hash_vec_size(handle: *mut BlockHashVecHandle) -> usize {
    (*handle).0.len()
}

#[no_mangle]
pub unsafe extern "C" fn rsn_block_hash_vec_push(handle: *mut BlockHashVecHandle, hash: *const u8) {
    (*handle).0.push(BlockHash::from_ptr(hash));
}

#[no_mangle]
pub unsafe extern "C" fn rsn_block_hash_vec_clear(handle: *mut BlockHashVecHandle) {
    (*handle).0.clear();
}

#[no_mangle]
pub unsafe extern "C" fn rsn_block_hash_vec_assign_range(
    destination: *mut BlockHashVecHandle,
    source: *const BlockHashVecHandle,
    start: usize,
    end: usize,
) {
    (*destination).0.clear();
    (*destination).0.extend_from_slice(&(*source).0[start..end]);
}

#[no_mangle]
pub unsafe extern "C" fn rsn_block_hash_vec_truncate(
    handle: *mut BlockHashVecHandle,
    new_size: usize,
) {
    (*handle).0.truncate(new_size);
}

pub struct AtomicU64Handle(pub Arc<AtomicU64>);

#[no_mangle]
pub extern "C" fn rsn_atomic_u64_create(value: u64) -> *mut AtomicU64Handle {
    Box::into_raw(Box::new(AtomicU64Handle(Arc::new(AtomicU64::new(value)))))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_atomic_u64_destroy(handle: *mut AtomicU64Handle) {
    drop(Box::from_raw(handle))
}

#[no_mangle]
pub unsafe extern "C" fn rsn_atomic_u64_load(handle: *mut AtomicU64Handle) -> u64 {
    (*handle).0.load(Ordering::SeqCst)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_atomic_u64_store(handle: *mut AtomicU64Handle, value: u64) {
    (*handle).0.store(value, Ordering::SeqCst)
}

#[no_mangle]
pub unsafe extern "C" fn rsn_atomic_u64_add(handle: *mut AtomicU64Handle, value: u64) {
    (*handle).0.fetch_add(value, Ordering::SeqCst);
}
